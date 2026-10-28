import {
  ArrowDownTrayIcon,
  ArrowPathIcon,
  ArrowPathRoundedSquareIcon,
  ArrowRightIcon,
  ArrowTopRightOnSquareIcon,
  ArrowUturnLeftIcon,
  CheckIcon,
  ChevronDownIcon,
  ChevronRightIcon,
  Cog6ToothIcon,
  CpuChipIcon,
  ExclamationCircleIcon,
  ExclamationTriangleIcon,
  FolderOpenIcon,
  HeartIcon,
  InformationCircleIcon,
  LanguageIcon,
  MicrophoneIcon,
  PaintBrushIcon,
  PlayIcon,
  ShieldCheckIcon,
  SparklesIcon,
  SpeakerWaveIcon,
  Square2StackIcon,
  StopIcon,
  SwatchIcon,
  WindowIcon,
  XMarkIcon,
} from '@heroicons/react/24/outline'
import type { ComponentType, SVGProps } from 'react'

export type IconName =
  | 'waveform'
  | 'speaker'
  | 'overlay'
  | 'translate'
  | 'chip'
  | 'settings'
  | 'keyboard'
  | 'info'
  | 'play'
  | 'stop'
  | 'refresh'
  | 'reset'
  | 'folder'
  | 'external'
  | 'alert'
  | 'error'
  | 'check'
  | 'close'
  | 'brush'
  | 'swatch'
  | 'border'
  | 'shield'
  | 'sparkle'
  | 'flow'
  | 'github'
  | 'heart'
  | 'download'
  | 'arrowRight'
  | 'chevron'
  | 'chevronDown'

type IconComponent = ComponentType<SVGProps<SVGSVGElement>>

function KeyboardGlyph(props: SVGProps<SVGSVGElement>) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      {...props}
    >
      <path d="M4 7h16a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V9a2 2 0 0 1 2-2Z" />
      <path d="M6 11h.01M9 11h.01M12 11h.01M15 11h.01M18 11h.01M8 14.5h8" />
    </svg>
  )
}

function GitHubGlyph(props: SVGProps<SVGSVGElement>) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" {...props}>
      <path d="M12 .3a12 12 0 0 0-3.8 23.4c.6.1.8-.3.8-.6v-2c-3.3.7-4-1.6-4-1.6-.6-1.4-1.4-1.8-1.4-1.8-1.1-.7.1-.7.1-.7 1.2.1 1.8 1.2 1.8 1.2 1.1 1.8 2.8 1.3 3.5 1 .1-.8.4-1.3.8-1.6-2.7-.3-5.5-1.3-5.5-5.9 0-1.3.5-2.4 1.2-3.2-.1-.3-.5-1.5.1-3.2 0 0 1-.3 3.3 1.2a11.5 11.5 0 0 1 6 0C16.9 4.2 18 4.5 18 4.5c.6 1.7.2 2.9.1 3.2a4.6 4.6 0 0 1 1.2 3.2c0 4.6-2.8 5.6-5.5 5.9.4.4.8 1.1.8 2.2v3.3c0 .3.2.7.8.6A12 12 0 0 0 12 .3" />
    </svg>
  )
}

const ICONS: Record<IconName, IconComponent> = {
  waveform: MicrophoneIcon,
  speaker: SpeakerWaveIcon,
  overlay: WindowIcon,
  translate: LanguageIcon,
  chip: CpuChipIcon,
  settings: Cog6ToothIcon,
  keyboard: KeyboardGlyph,
  info: InformationCircleIcon,
  play: PlayIcon,
  stop: StopIcon,
  refresh: ArrowPathIcon,
  reset: ArrowUturnLeftIcon,
  folder: FolderOpenIcon,
  external: ArrowTopRightOnSquareIcon,
  alert: ExclamationTriangleIcon,
  error: ExclamationCircleIcon,
  check: CheckIcon,
  close: XMarkIcon,
  brush: PaintBrushIcon,
  swatch: SwatchIcon,
  border: Square2StackIcon,
  shield: ShieldCheckIcon,
  sparkle: SparklesIcon,
  flow: ArrowPathRoundedSquareIcon,
  github: GitHubGlyph,
  heart: HeartIcon,
  download: ArrowDownTrayIcon,
  arrowRight: ArrowRightIcon,
  chevron: ChevronRightIcon,
  chevronDown: ChevronDownIcon,
}

interface Props {
  name: IconName
  size?: number
  strokeWidth?: number
  className?: string
}

export function Icon({ name, size = 16, strokeWidth, className }: Props) {
  const Glyph = ICONS[name]
  return <Glyph width={size} height={size} strokeWidth={strokeWidth} className={className} aria-hidden="true" />
}
