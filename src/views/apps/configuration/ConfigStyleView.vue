<script lang="ts" setup>
import ConfigAppLayout from '@/layouts/ConfigAppLayout.vue';

</script>

<template>
  <ConfigAppLayout>
    <div class="config-style-view">
      <h2 class="view-title">Configuración de Estilos</h2>
      <p class="description">
        Los estilos visuales como colores, border radius y temas se configuran en la sección de 
        <strong>Sistema Operativo</strong> (⚙️) del menú de configuración.
      </p>
      <p class="description secondary">
        Accede a <router-link to="/apps/configuration/system">Configuración del Sistema</router-link> 
        para personalizar la apariencia de VasakOS.
      </p>
    </div>
  </ConfigAppLayout>
</template>

<style scoped>
.config-style-view {
  padding: 24px;
  max-width: 600px;
}

.view-title {
  font-size: 24px;
  font-weight: 600;
  margin-bottom: 16px;
  color: var(--text-primary, #fff);
}

.description {
  font-size: 16px;
  line-height: 1.6;
  color: var(--text-secondary, rgba(255, 255, 255, 0.8));
  margin-bottom: 12px;
}

.description.secondary {
  font-size: 14px;
  opacity: 0.9;
}

.description strong {
  color: var(--primary-color, #0084FF);
  font-weight: 600;
}

.description a {
  color: var(--primary-color, #0084FF);
  text-decoration: none;
  transition: opacity 0.2s ease;
}

.description a:hover {
  opacity: 0.8;
  text-decoration: underline;
}
</style>
