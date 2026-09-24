<script lang="ts" setup>
import { computed, onMounted, onUnmounted, ref } from 'vue';

const now = ref(new Date());

/**
 * La hora sin segundos, y la fecha abajo.
 *
 * Con segundos el texto tenía ocho caracteres de ancho fijo, y en la celda de
 * siempre no entraba: se cortaba por los costados. Horas y minutos es lo que se
 * lee de un vistazo en el escritorio; el que necesite los segundos los tiene en
 * el panel.
 */
const time = computed(() =>
	now.value.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })
);
const date = computed(() =>
	now.value.toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'long' })
);

// The timer used to be created at setup scope and never cleared, so it kept
// running — and kept the component alive — after the widget was gone.
let tick: ReturnType<typeof setInterval> | undefined;

const updateTime = () => {
	now.value = new Date();
};

onMounted(() => {
	updateTime();
	tick = setInterval(updateTime, 1000);
});

onUnmounted(() => {
	if (tick !== undefined) clearInterval(tick);
});
</script>

<template>
    <!-- Las medidas van contra los dos lados de la celda —`cqh` y `cqw`— y gana
         la más chica. Medir sólo el alto era lo que cortaba la hora en una
         celda angosta: la letra crecía con la fila y el texto se salía por los
         costados. -->
    <div class="flex h-full w-full flex-col items-center justify-center gap-[2cqh] p-[4cqmin] text-center">
        <h1
            class="whitespace-nowrap font-mono font-bold leading-none tabular-nums text-tx-main"
            style="font-size: min(42cqh, 19cqw)"
        >
            {{ time }}
        </h1>
        <p
            class="max-w-full truncate font-medium leading-tight text-tx-muted first-letter:uppercase"
            style="font-size: min(12cqh, 6cqw)"
        >
            {{ date }}
        </p>
    </div>
</template>
