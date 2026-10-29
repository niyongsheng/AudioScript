import { useCallback, useEffect, useState } from 'react'
import { EVENT, formatCmdError, type ModelJobStatus, ModelPhase, type ModelsList, modelsList, on } from '@/shared/ipc'

interface ModelJobFailure {
  modelId: string
  message: string | null
}

interface ModelsState {
  list: ModelsList | null
  setList: (list: ModelsList) => void
  job: ModelJobStatus | null
  loadError: string | null
  failure: ModelJobFailure | null
  dismissFailure: () => void
  clearLoadError: () => void
  refresh: () => Promise<void>
}

function isTerminalPhase(phase: ModelPhase): boolean {
  return phase === ModelPhase.Done || phase === ModelPhase.Failed || phase === ModelPhase.Cancelled
}

export function useModels(): ModelsState {
  const [list, setList] = useState<ModelsList | null>(null)
  const [job, setJob] = useState<ModelJobStatus | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [failure, setFailure] = useState<ModelJobFailure | null>(null)

  const refresh = useCallback(async () => {
    try {
      const next = await modelsList()
      setList(next)
      setJob(next.job ?? null)
      setLoadError(null)
    } catch (e) {
      setLoadError(formatCmdError(e))
    }
  }, [])

  useEffect(() => {
    let disposed = false
    void refresh()

    const unlisten = on<ModelJobStatus>(EVENT.modelsProgress, (payload) => {
      if (disposed) return
      if (!isTerminalPhase(payload.phase)) {
        setJob(payload)
        return
      }
      if (payload.phase === ModelPhase.Failed) {
        setFailure({ modelId: payload.modelId, message: payload.error ?? null })
      }
      setJob(null)
      void refresh()
    })

    return () => {
      disposed = true
      void unlisten.then((fn) => fn())
    }
  }, [refresh])

  const dismissFailure = useCallback(() => setFailure(null), [])
  const clearLoadError = useCallback(() => setLoadError(null), [])

  return { list, setList, job, loadError, failure, dismissFailure, clearLoadError, refresh }
}
