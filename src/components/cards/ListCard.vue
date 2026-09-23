<script setup lang="ts">
interface Props {
	clickable?: boolean;
	customClass?: string | Record<string, boolean>;
}

const props = withDefaults(defineProps<Props>(), {
	clickable: false,
	customClass: () => ({}),
});

const emit = defineEmits<{
	click: [];
}>();

const handleClick = () => {
	if (props.clickable) {
		emit('click');
	}
};
</script>

<template>
  <div
    :class="[
      'flex items-center justify-between bg-ui-bg/80 p-3 rounded-vsk border border-vsk-primary/70 transition-colors duration-200',
      {
        'hover:bg-vsk-primary/5 cursor-pointer': props.clickable,
      },
      customClass,
    ]"
    :role="props.clickable ? 'button' : undefined"
    :tabindex="props.clickable ? 0 : undefined"
    @click="handleClick"
    @keydown.enter.prevent="handleClick"
    @keydown.space.prevent="handleClick"
  >
    <!-- `role="button"` y no un `<button>`: lo que entra por la ranura decide
         quien lo usa, y ya hay quien mete botones adentro. El nombre accesible
         sale de ese mismo contenido. -->
    <slot />
  </div>
</template>
