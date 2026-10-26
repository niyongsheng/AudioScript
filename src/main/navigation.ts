import { t } from '@/shared/i18n'
import type { SidebarItem } from '@/ui/Sidebar'

export type PageId = 'transcribe' | 'audio' | 'permissions' | 'translate' | 'overlay' | 'general' | 'models' | 'about'

export function navItems(): SidebarItem<PageId>[] {
  return [
    {
      id: 'transcribe',
      label: t('nav.transcribe'),
      icon: 'waveform',
    },
    {
      id: 'audio',
      label: t('nav.audio'),
      icon: 'speaker',
    },
    { id: 'permissions', label: t('nav.permissions'), icon: 'shield' },
    {
      id: 'translate',
      label: t('nav.translate'),
      icon: 'translate',
    },
    {
      id: 'overlay',
      label: t('nav.overlay'),
      icon: 'overlay',
    },
    { id: 'general', label: t('nav.general'), icon: 'settings' },
    { id: 'models', label: t('nav.models'), icon: 'chip' },
    { id: 'about', label: t('nav.about'), icon: 'info' },
  ]
}
