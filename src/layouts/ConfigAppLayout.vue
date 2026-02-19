<script setup lang="ts">
</script>

<template>
  <WindowFrame class="rounded-t-window">
    <div class="h-[calc(100vh-32px)] w-screen rounded-b-window background">
      <div class="h-full w-full flex p-4 gap-4">
        <ConfigSidebarArea />
        <div class="flex-1 min-h-0">
          <div class="h-full overflow-auto">
            <slot />
          </div>
        </div>
      </div>
    </div>
  </WindowFrame>
</template>
