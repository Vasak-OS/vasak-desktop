<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
/**
 * El indicador de «No molestar» en la bandeja (vasak-desktop#177). Sólo se ve
 * con el modo puesto, como el de Bloq Mayús: avisa que los carteles no van a
 * salir, que es lo que se olvida.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { TrayIconButton } from '@vasakgroup/vue-libvasak';
import { useDoNotDisturb } from '@/tools/composables/useDoNotDisturb';

const { t } = useI18n();
const { enabled } = useDoNotDisturb();
</script>

<template>
  <TrayIconButton
    v-if="enabled"
    name="notifications-disabled-symbolic"
    :tooltip="t('components.TrayIconDoNotDisturb.label')"
    :alt="t('components.TrayIconDoNotDisturb.label')"
    :interactive="false"
    data-do-not-disturb
  />
</template>
