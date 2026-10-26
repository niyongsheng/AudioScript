import { PermissionPanel } from '@/main/permissions/PermissionPanel'
import { useI18n } from '@/shared/i18n'
import { Page } from '@/ui/Page'

export function PermissionsPage() {
  const { t } = useI18n()
  return (
    <Page title={t('permissions.title')}>
      <PermissionPanel />
    </Page>
  )
}
