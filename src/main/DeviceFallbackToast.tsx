import { useCallback, useEffect, useState } from 'react'
import { useI18n } from '@/shared/i18n'
import { EVENT, on } from '@/shared/ipc'
import { AudioSourceKind, type DeviceFallbackEvent, type InputDevice } from '@/shared/types'
import { Alert } from '@/ui/Alert'
import { useStore } from './store'

interface Props {
  hidden: boolean
}

/** Notice for capture devices that disappeared and fell back to the OS default device. */
export function DeviceFallbackToast({ hidden }: Props) {
  const { t } = useI18n()
  const {
    audioSource,
    device,
    devices,
    mixMicDevice,
    mixMicDevices,
    mixSystemDevice,
    mixSystemDevices,
    setDevice,
    setMixMicDevice,
    setMixSystemDevice,
  } = useStore()
  const [notice, setNotice] = useState<DeviceFallbackEvent | null>(null)

  const dismiss = useCallback(() => setNotice(null), [])

  useEffect(() => {
    let cancelled = false
    let unlisten: (() => void) | undefined
    void on<DeviceFallbackEvent>(EVENT.deviceFallback, (payload) => {
      setNotice(payload)
      if (audioSource === AudioSourceKind.Mix) {
        if (payload.requested.includes(mixMicDevice)) setMixMicDevice('')
        if (payload.requested.includes(mixSystemDevice)) setMixSystemDevice('')
      } else if (payload.requested.includes(device)) {
        setDevice('')
      }
    }).then((fn) => {
      if (cancelled) fn()
      else unlisten = fn
    })
    return () => {
      cancelled = true
      unlisten?.()
    }
  }, [audioSource, device, mixMicDevice, mixSystemDevice, setDevice, setMixMicDevice, setMixSystemDevice])

  useEffect(() => {
    if (audioSource === AudioSourceKind.App) return
    if (audioSource === AudioSourceKind.Mix) {
      if (mixMicDevices.length > 0 && mixMicDevice && !mixMicDevices.some((d) => d.id === mixMicDevice)) {
        setMixMicDevice('')
        setNotice({ requested: [mixMicDevice], actual: defaultDeviceName(mixMicDevices) })
      }
      if (mixSystemDevices.length > 0 && mixSystemDevice && !mixSystemDevices.some((d) => d.id === mixSystemDevice)) {
        setMixSystemDevice('')
        setNotice({ requested: [mixSystemDevice], actual: defaultDeviceName(mixSystemDevices) })
      }
      return
    }
    if (devices.length > 0 && device && !devices.some((d) => d.id === device)) {
      setDevice('')
      setNotice({ requested: [device], actual: defaultDeviceName(devices) })
    }
  }, [
    audioSource,
    device,
    devices,
    mixMicDevice,
    mixMicDevices,
    mixSystemDevice,
    mixSystemDevices,
    setDevice,
    setMixMicDevice,
    setMixSystemDevice,
  ])

  useEffect(() => {
    if (!notice) return
    const timer = window.setTimeout(() => setNotice(null), 12000)
    return () => window.clearTimeout(timer)
  }, [notice])

  if (hidden || !notice) return null

  const actual = notice.actual || t('audio.deviceDefaultLabel')
  return (
    <div className="fixed start-5 bottom-5 z-[60] w-80 max-w-[calc(100vw-2.5rem)]">
      <Alert
        tone="info"
        className="shadow-[0_12px_40px_rgb(0_0_0/0.35)]"
        title={t('audio.deviceFallbackTitle')}
        onDismiss={dismiss}
      >
        {t('audio.deviceFallbackBody', { device: actual })}
      </Alert>
    </div>
  )
}

function defaultDeviceName(list: InputDevice[]): string {
  return list.find((d) => d.isDefault)?.name ?? ''
}
