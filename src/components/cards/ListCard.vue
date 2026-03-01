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
      'flex items-center justify-between background p-3 rounded-vsk border border-vsk-primary/70 transition-colors duration-200',
      {
        'hover:bg-vsk-primary/5 cursor-pointer': props.clickable,
      },
      customClass,
    ]"
    @click="handleClick"
  >
    <slot />
  </div>
</template>
