<template>
  <div
    class="flex items-center justify-between background rounded-vsk px-6 py-3 mb-4"
    :class="[{ 'border-l-4 border-green-500': isConnected }, customClass]"
    @click="handleClick"
  >
    <div class="flex items-center gap-3 flex-1 min-w-0">
      <img :src="icon" :alt="title" class="h-7 w-7 shrink-0" />
      <div class="min-w-0">
        <div class="font-semibold truncate">
          {{ title }}
        </div>
        <div v-if="subtitle" class="text-xs text-gray-400 truncate">
          {{ subtitle }}
        </div>
        <div v-if="metadata" class="text-xs text-gray-400 truncate">
          {{ metadata }}
        </div>
        <div v-if="extraInfo && extraInfo.length > 0" class="text-xs text-gray-400 flex gap-2 mt-1">
          <span v-for="(info, index) in extraInfo" :key="index">
            {{ info }}
          </span>
        </div>
      </div>
    </div>
    
    <button
      v-if="showActionButton"
      class="bg-vsk-primary rounded-vsk px-4 py-2 text-sm font-semibold cursor-pointer hover:opacity-70"
      @click.stop="handleAction"
    >
      {{ actionLabel }}
    </button>

    <!-- Status indicator for connected state -->
    <div
      v-if="showStatusIndicator && isConnected"
      class="w-2 h-2 rounded-full bg-green-500"
    />
  </div>
</template>

<script setup lang="ts">
interface Props {
	icon: string;
	title: string;
	subtitle?: string;
	metadata?: string;
	extraInfo?: string[];
	isConnected?: boolean;
	showActionButton?: boolean;
	actionLabel?: string;
	showStatusIndicator?: boolean;
	customClass?: string;
	clickable?: boolean;
}

withDefaults(defineProps<Props>(), {
	subtitle: '',
	metadata: '',
	extraInfo: () => [],
	isConnected: false,
	showActionButton: true,
	actionLabel: 'Conectar',
	showStatusIndicator: false,
	customClass: '',
	clickable: false,
});

const emit = defineEmits<{
	action: [];
	click: [];
}>();

const handleAction = () => {
	emit('action');
};

const handleClick = () => {
	emit('click');
};
</script>
