import AVFoundation
import Foundation
import os
import Speech

private let kUnsupported: Int32 = -1
private let kOk: Int32 = 0
private let kError: Int32 = 1

private func osSupported() -> Bool {
    if #available(macOS 26.0, *) {
        #if arch(arm64)
        return true
        #else
        return false
        #endif
    }
    return false
}

private func runningFromAppBundle() -> Bool {
    Bundle.main.bundleURL.pathExtension == "app"
}

private func usageDescriptionPresent() -> Bool {
    guard runningFromAppBundle() else { return false }
    guard let value = Bundle.main.object(forInfoDictionaryKey: "NSSpeechRecognitionUsageDescription") as? String else {
        return false
    }
    return !value.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
}

private func cString(_ value: String) -> UnsafeMutablePointer<CChar>? {
    value.withCString { strdup($0) }
}

private func setError(_ out: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?, _ message: String) {
    out?.pointee = cString(message)
}

private struct FeedItem: Sendable {
    let index: Int
    let text: String
    let isFinal: Bool
    let startMs: Int
    let endMs: Int
}

private struct SegmentSlot: Sendable {
    let startMs: Int
    let index: Int
    var closed: Bool
}

private struct BridgeState: Sendable {
    var pending: [FeedItem] = []
    var segments: [SegmentSlot] = []
    var nextIndex = 0
    var error: String?
    var finished = false
}

private enum BridgeError: Error, CustomStringConvertible {
    case noFormat
    case unsupportedLocale(String)
    case assetsMissing(String, [String])
    case audio(String)
    case results(String)

    var description: String {
        switch self {
        case .noFormat:
            return "SpeechAnalyzer did not provide an audio format"
        case .unsupportedLocale(let id):
            return "locale not supported: \(id)"
        case .assetsMissing(let id, let installed):
            return "speech assets not installed for \(id); installed: \(installed.joined(separator: ", "))"
        case .audio(let message):
            return "audio conversion failed: \(message)"
        case .results(let message):
            return "result stream failed: \(message)"
        }
    }
}

private func waitFor<T>(_ operation: @escaping @Sendable () async throws -> T) throws -> T {
    let semaphore = DispatchSemaphore(value: 0)
    let box = ResultBox<T>()
    Task {
        do {
            box.set(.success(try await operation()))
        } catch {
            box.set(.failure(error))
        }
        semaphore.signal()
    }
    semaphore.wait()
    return try box.get()
}

private final class ResultBox<T>: @unchecked Sendable {
    private let lock = OSAllocatedUnfairLock<Result<T, Error>?>(initialState: nil)

    func set(_ value: Result<T, Error>) {
        lock.withLock { $0 = value }
    }

    func get() throws -> T {
        guard let value = lock.withLock({ $0 }) else {
            throw BridgeError.results("operation did not produce a result")
        }
        return try value.get()
    }
}

@available(macOS 26.0, *)
private final class SpeechBridgeSession: @unchecked Sendable {
    let locale: Locale
    let transcriber: SpeechTranscriber
    let analyzer: SpeechAnalyzer
    let format: AVAudioFormat
    let converter: AVAudioConverter?
    let inputFormat: AVAudioFormat
    let stream: AsyncStream<AnalyzerInput>
    let continuation: AsyncStream<AnalyzerInput>.Continuation
    let state = OSAllocatedUnfairLock(initialState: BridgeState())
    let resultsDone = DispatchSemaphore(value: 0)
    var resultsTask: Task<Void, Never>?
    var finalized = false

    private init(locale: Locale, format: AVAudioFormat, transcriber: SpeechTranscriber) {
        self.locale = locale
        self.transcriber = transcriber
        analyzer = SpeechAnalyzer(modules: [transcriber])
        self.format = format
        inputFormat = AVAudioFormat(standardFormatWithSampleRate: 16_000, channels: 1)!
        converter = AVAudioConverter(from: inputFormat, to: format)
        (stream, continuation) = AsyncStream<AnalyzerInput>.makeStream(bufferingPolicy: .bufferingNewest(64))
    }

    static func make(language: String) throws -> SpeechBridgeSession {
        let supported = try waitFor { await SpeechTranscriber.supportedLocales }
        let installed = try waitFor { await SpeechTranscriber.installedLocales }

        guard let locale = resolveLocale(language: language, supported: supported, installed: installed) else {
            throw BridgeError.unsupportedLocale(language)
        }
        guard installed.contains(where: { $0.identifier == locale.identifier }) else {
            let names = installed.map(\.identifier).sorted()
            throw BridgeError.assetsMissing(locale.identifier, names)
        }

        _ = try waitFor { try await AssetInventory.reserve(locale: locale) }

        let transcriber = SpeechTranscriber(
            locale: locale,
            transcriptionOptions: [],
            reportingOptions: [.volatileResults, .fastResults],
            attributeOptions: [.audioTimeRange]
        )
        guard let format = try waitFor({ await SpeechAnalyzer.bestAvailableAudioFormat(compatibleWith: [transcriber]) }) else {
            throw BridgeError.noFormat
        }

        let session = SpeechBridgeSession(locale: locale, format: format, transcriber: transcriber)
        try session.start()
        return session
    }

    private static func resolveLocale(language: String, supported: [Locale], installed: [Locale]) -> Locale? {
        let base = language.split(separator: "-").first.map(String.init)?.lowercased() ?? language.lowercased()
        let candidates = supported.filter { locale in
            let code = locale.language.languageCode?.identifier ?? locale.identifier.split(separator: "_").first.map(String.init) ?? ""
            return code.lowercased() == base
        }
        guard !candidates.isEmpty else { return nil }

        let installedIds = Set(installed.map(\.identifier))
        let installedCandidates = candidates.filter { installedIds.contains($0.identifier) }

        let preferred: [String]
        switch base {
        case "en": preferred = ["en_US", "en_GB"]
        case "zh": preferred = ["zh_CN", "zh_TW", "zh_HK"]
        default: preferred = []
        }

        for id in preferred {
            if let match = installedCandidates.first(where: { $0.identifier == id }) {
                return match
            }
        }
        return installedCandidates.first ?? candidates.first
    }

    private func start() throws {
        try waitFor { try await self.analyzer.start(inputSequence: self.stream) }
        resultsTask = Task { [self] in
            do {
                for try await result in self.transcriber.results {
                    let text = String(result.text.characters).trimmingCharacters(in: .whitespacesAndNewlines)
                    guard !text.isEmpty else { continue }
                    let startMs = max(0, Int(result.range.start.seconds * 1000))
                    let endMs = max(startMs, Int(result.range.end.seconds * 1000))
                    let index = state.withLock { current -> Int in
                        current.allocate(startMs: startMs, closed: result.isFinal)
                    }
                    state.withLock {
                        $0.pending.append(FeedItem(index: index, text: text, isFinal: result.isFinal, startMs: startMs, endMs: endMs))
                    }
                }
            } catch {
                state.withLock { $0.error = BridgeError.results(error.localizedDescription).description }
            }
            state.withLock { $0.finished = true }
            resultsDone.signal()
        }
    }

    func feed(samples: UnsafePointer<Float>, count: Int) throws {
        if let error = state.withLock({ $0.error }) {
            throw BridgeError.results(error)
        }
        guard count > 0 else { return }

        if let converter {
            guard let input = AVAudioPCMBuffer(pcmFormat: inputFormat, frameCapacity: AVAudioFrameCount(count)) else {
                throw BridgeError.audio("cannot allocate input buffer")
            }
            input.frameLength = AVAudioFrameCount(count)
            if let channel = input.floatChannelData?[0] {
                channel.update(from: samples, count: count)
            }
            let ratio = format.sampleRate / inputFormat.sampleRate
            let capacity = AVAudioFrameCount(Double(count) * ratio) + 64
            guard let output = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: capacity) else {
                throw BridgeError.audio("cannot allocate output buffer")
            }
            var supplied = false
            var error: NSError?
            let status = converter.convert(to: output, error: &error) { _, outStatus in
                if supplied {
                    outStatus.pointee = .noDataNow
                    return nil
                }
                supplied = true
                outStatus.pointee = .haveData
                return input
            }
            if status == .error {
                throw BridgeError.audio(error?.localizedDescription ?? "unknown")
            }
            if output.frameLength > 0 {
                continuation.yield(AnalyzerInput(buffer: output))
            }
            return
        }

        guard let buffer = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: AVAudioFrameCount(count)) else {
            throw BridgeError.audio("cannot allocate buffer")
        }
        buffer.frameLength = AVAudioFrameCount(count)
        switch format.commonFormat {
        case .pcmFormatFloat32:
            buffer.floatChannelData?[0].update(from: samples, count: count)
        case .pcmFormatInt16:
            guard let destination = buffer.int16ChannelData?[0] else {
                throw BridgeError.audio("missing int16 channel")
            }
            for i in 0..<count {
                destination[i] = Int16(max(-1, min(1, samples[i])) * 32_767)
            }
        default:
            throw BridgeError.audio("unsupported analyzer format")
        }
        continuation.yield(AnalyzerInput(buffer: buffer))
    }

    func poll() -> String {
        let items = state.withLock { current -> [FeedItem] in
            let out = current.pending
            current.pending.removeAll(keepingCapacity: true)
            return out
        }
        guard !items.isEmpty else { return "" }
        let objects: [[String: Any]] = items.map {
            ["i": $0.index, "t": $0.text, "f": $0.isFinal, "s": $0.startMs, "e": $0.endMs]
        }
        guard let data = try? JSONSerialization.data(withJSONObject: objects),
              let json = String(data: data, encoding: .utf8) else {
            return "[]"
        }
        return json
    }

    func flush(timeoutMs: Int) throws -> String {
        if !finalized {
            finalized = true
            continuation.finish()
            let semaphore = DispatchSemaphore(value: 0)
            Task {
                do {
                    try await self.analyzer.finalizeAndFinishThroughEndOfInput()
                } catch {
                    self.state.withLock { $0.error = BridgeError.results(error.localizedDescription).description }
                }
                semaphore.signal()
            }
            _ = semaphore.wait(timeout: .now() + .milliseconds(timeoutMs))
        }
        _ = resultsDone.wait(timeout: .now() + .milliseconds(timeoutMs))
        if let error = state.withLock({ $0.error }) {
            throw BridgeError.results(error)
        }
        return poll()
    }

    func shutdown() {
        resultsTask?.cancel()
        continuation.finish()
        Task { await analyzer.cancelAndFinishNow() }
        Task { await AssetInventory.release(reservedLocale: self.locale) }
    }
}

private extension BridgeState {
    mutating func allocate(startMs: Int, closed: Bool) -> Int {
        let tolerance = 500
        if let slotIndex = segments.indices
            .filter({ !segments[$0].closed })
            .min(by: { abs(segments[$0].startMs - startMs) < abs(segments[$1].startMs - startMs) }),
           abs(segments[slotIndex].startMs - startMs) <= tolerance {
            let index = segments[slotIndex].index
            if closed {
                segments[slotIndex].closed = true
            }
            return index
        }
        let index = nextIndex
        nextIndex += 1
        segments.append(SegmentSlot(startMs: startMs, index: index, closed: closed))
        return index
    }
}

@_cdecl("audioscript_speech_available")
public func audioscript_speech_available() -> Int32 {
    guard osSupported() else { return kUnsupported }
    guard usageDescriptionPresent() else { return -2 }
    return 1
}

@_cdecl("audioscript_speech_authorization")
public func audioscript_speech_authorization() -> Int32 {
    guard osSupported() else { return kUnsupported }
    return Int32(SFSpeechRecognizer.authorizationStatus().rawValue)
}

@_cdecl("audioscript_speech_request_authorization")
public func audioscript_speech_request_authorization(_ timeoutMs: Int32) -> Int32 {
    guard osSupported(), usageDescriptionPresent() else { return kUnsupported }
    if SFSpeechRecognizer.authorizationStatus() != .notDetermined {
        return Int32(SFSpeechRecognizer.authorizationStatus().rawValue)
    }
    let semaphore = DispatchSemaphore(value: 0)
    SFSpeechRecognizer.requestAuthorization { _ in semaphore.signal() }
    _ = semaphore.wait(timeout: .now() + .milliseconds(max(0, Int(timeoutMs))))
    return Int32(SFSpeechRecognizer.authorizationStatus().rawValue)
}

@_cdecl("audioscript_speech_locales")
public func audioscript_speech_locales(_ installedOnly: Int32) -> UnsafeMutablePointer<CChar>? {
    guard osSupported(), #available(macOS 26.0, *) else { return cString("[]") }
    let locales = try? waitFor { () -> [Locale] in
        if installedOnly != 0 {
            return await SpeechTranscriber.installedLocales
        }
        return await SpeechTranscriber.supportedLocales
    }
    let names = (locales ?? []).map(\.identifier).sorted()
    guard let data = try? JSONSerialization.data(withJSONObject: names),
          let json = String(data: data, encoding: .utf8) else {
        return cString("[]")
    }
    return cString(json)
}

@_cdecl("audioscript_speech_create")
public func audioscript_speech_create(
    _ language: UnsafePointer<CChar>,
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?,
    _ kindOut: UnsafeMutablePointer<Int32>?
) -> UnsafeMutableRawPointer? {
    kindOut?.pointee = 5
    guard osSupported(), #available(macOS 26.0, *) else {
        kindOut?.pointee = 1
        setError(errorOut, "macOS 26+ on Apple Silicon is required")
        return nil
    }
    guard usageDescriptionPresent() else {
        kindOut?.pointee = 2
        setError(errorOut, "NSSpeechRecognitionUsageDescription is missing; run the packaged app")
        return nil
    }
    let languageString = String(cString: language)
    do {
        let session = try SpeechBridgeSession.make(language: languageString)
        kindOut?.pointee = 0
        return Unmanaged.passRetained(session).toOpaque()
    } catch BridgeError.unsupportedLocale(let id) {
        kindOut?.pointee = 3
        setError(errorOut, "locale not supported: \(id)")
        return nil
    } catch BridgeError.assetsMissing(let id, let installed) {
        kindOut?.pointee = 4
        setError(errorOut, "speech assets not installed for \(id); installed: \(installed.joined(separator: ", "))")
        return nil
    } catch let error as BridgeError {
        kindOut?.pointee = 5
        setError(errorOut, error.description)
        return nil
    } catch {
        kindOut?.pointee = 5
        setError(errorOut, error.localizedDescription)
        return nil
    }
}

@_cdecl("audioscript_speech_feed")
public func audioscript_speech_feed(
    _ handle: UnsafeMutableRawPointer?,
    _ samples: UnsafePointer<Float>?,
    _ count: Int32,
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    guard #available(macOS 26.0, *) else {
        setError(errorOut, "macOS 26+ is required")
        return kError
    }
    guard let handle, let samples, count > 0 else {
        setError(errorOut, "invalid feed arguments")
        return kError
    }
    let session = Unmanaged<SpeechBridgeSession>.fromOpaque(handle).takeUnretainedValue()
    do {
        try session.feed(samples: samples, count: Int(count))
        return kOk
    } catch let error as BridgeError {
        setError(errorOut, error.description)
        return kError
    } catch {
        setError(errorOut, error.localizedDescription)
        return kError
    }
}

@_cdecl("audioscript_speech_poll")
public func audioscript_speech_poll(
    _ handle: UnsafeMutableRawPointer?,
    _ outJson: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    guard #available(macOS 26.0, *) else {
        outJson.pointee = cString("[]")
        setError(errorOut, "macOS 26+ is required")
        return kError
    }
    guard let handle else {
        outJson.pointee = cString("[]")
        setError(errorOut, "invalid session handle")
        return kError
    }
    let session = Unmanaged<SpeechBridgeSession>.fromOpaque(handle).takeUnretainedValue()
    if let error = session.state.withLock({ $0.error }) {
        outJson.pointee = cString("[]")
        setError(errorOut, error)
        return kError
    }
    outJson.pointee = cString(session.poll())
    return kOk
}

@_cdecl("audioscript_speech_flush")
public func audioscript_speech_flush(
    _ handle: UnsafeMutableRawPointer?,
    _ timeoutMs: Int32,
    _ outJson: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>,
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    guard #available(macOS 26.0, *) else {
        outJson.pointee = cString("[]")
        setError(errorOut, "macOS 26+ is required")
        return kError
    }
    guard let handle else {
        outJson.pointee = cString("[]")
        setError(errorOut, "invalid session handle")
        return kError
    }
    let session = Unmanaged<SpeechBridgeSession>.fromOpaque(handle).takeUnretainedValue()
    do {
        let json = try session.flush(timeoutMs: Int(timeoutMs))
        outJson.pointee = cString(json)
        return kOk
    } catch let error as BridgeError {
        outJson.pointee = cString("[]")
        setError(errorOut, error.description)
        return kError
    } catch {
        outJson.pointee = cString("[]")
        setError(errorOut, error.localizedDescription)
        return kError
    }
}

@_cdecl("audioscript_speech_destroy")
public func audioscript_speech_destroy(_ handle: UnsafeMutableRawPointer?) {
    guard #available(macOS 26.0, *) else { return }
    guard let handle else { return }
    Unmanaged<SpeechBridgeSession>.fromOpaque(handle).takeRetainedValue().shutdown()
}

@_cdecl("audioscript_speech_free")
public func audioscript_speech_free(_ pointer: UnsafeMutablePointer<CChar>?) {
    free(pointer)
}
