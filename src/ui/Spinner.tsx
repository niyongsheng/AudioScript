import { cn } from './cn'

interface Props {
  size?: number
  className?: string
}

const BLADES = [0, 45, 90, 135, 180, 225, 270, 315]

export function Spinner({ size = 14, className }: Props) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      role="presentation"
      aria-hidden="true"
      className={cn('origin-center animate-spin', className)}
    >
      {BLADES.map((angle, i) => (
        <rect
          key={angle}
          x="11"
          y="2"
          width="2"
          height="6"
          rx="1"
          fill="currentColor"
          opacity={0.12 + (i * 0.88) / 7}
          transform={`rotate(${angle} 12 12)`}
        />
      ))}
    </svg>
  )
}
