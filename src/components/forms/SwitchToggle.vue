<template>
  <button
    type="button"
    role="switch"
    :aria-checked="isOn"
    :aria-label="label"
    @click="handleClick"
    :disabled="disabled"
    :class="[
      'relative inline-flex items-center rounded-full transition-colors',
      size === 'small' ? 'h-6 w-11' : 'h-7 w-12',
      isOn ? activeClass : inactiveClass,
      disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer',
      customClass
    ]"
  >
    <span
      :class="[
        'inline-block transform rounded-full shadow transition-transform',
        // El pulgar es el primer plano de su vía: con `bg-white` fijo daba 1.54
        // sobre la vía apagada en modo claro —contra el mínimo de 3.0 de WCAG
        // 1.4.11— así que el estado del interruptor no se percibía. `tx-on-primary`
        // está garantizado sobre el acento, sea el que sea.
        isOn ? 'bg-tx-on-primary' : 'bg-tx-main',
        size === 'small' ? 'h-4 w-4' : 'h-6 w-6',
        isOn ? (size === 'small' ? 'translate-x-6' : 'translate-x-5') : 'translate-x-1'
      ]"
    ></span>
  </button>
</template>

<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
/**
 * Un interruptor de dos estados.
 *
 * `label` es **obligatorio** y no un extra: un botón que sólo tiene un círculo
 * adentro no tiene nombre, y un lector de pantalla anuncia «botón» y nada más.
 * Como la etiqueta ya está escrita al lado del interruptor en todos los usos, el
 * consumidor pasa la misma cadena y no hay que inventar nada.
 *
 * Y va con `role="switch"` más `aria-checked`: sin eso, un `<button>` se anuncia
 * como botón y no dice si está encendido o apagado, que es la única información
 * que este control transmite.
 */
interface Props {
	/** Qué controla este interruptor. Es su nombre accesible. */
	label: string;
	isOn: boolean;
	disabled?: boolean;
	size?: 'small' | 'medium';
	activeClass?: string;
	inactiveClass?: string;
	customClass?: string;
}

const props = withDefaults(defineProps<Props>(), {
	disabled: false,
	size: 'small',
	activeClass: 'bg-primary',
	inactiveClass: 'bg-ui-bg/80',
	customClass: '',
});

const emit = defineEmits<{
	toggle: [value: boolean];
}>();

const handleClick = () => {
	emit('toggle', !props.isOn);
};
</script>
