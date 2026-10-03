<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import { PanelPill, SpinningCover } from '@vasakgroup/vue-libvasak';
import { computed, onMounted, ref } from 'vue';
import { toggleApplet } from '@/services/window.service';
import { useMusicPlayer } from '@/tools/composables/useMusicPlayer';
import { useOpenApplet } from '@/tools/composables/useOpenApplet';
import { usePanelConfig } from '@/tools/composables/usePanelConfig';
import { logError } from '@/utils/logger';
import { formatDuration, playbackStateOf } from '@/utils/playback';

/**
 * El control de música del panel: un indicador que abre el reproductor.
 *
 * La píldora entera es un botón —la portada que gira con su aro de avance y el
 * título corto al lado— y tocarla despliega el reproductor anclado debajo
 * (`MusicAppletView`, el applet `music` de `APPLETS`), con el mismo mecanismo
 * que los demás applets del panel (#134). Los comandos —anterior, reproducir,
 * siguiente, la barra, la salida— viven **en el reproductor**, no acá: hasta
 * 1.23 aparecían en el panel al pasar el puntero, y lo único que abría el
 * reproductor era la portada de 22 píxeles (vasak-desktop#131).
 *
 * Con el panel a un costado no hay lugar para el título: queda la portada.
 *
 * En el panel en píldoras (vasak-desktop#151) es una `PanelPill` como las
 * demás: la portada mini, el título cortado y, chico debajo, por dónde va
 * —«01:42 / 04:19»—, como en el video de referencia. Sigue sin comandos.
 */

const { t } = useI18n();
const { vertical } = usePanelConfig();

const { musicInfo, imgSrc, position, progress, onImgError, initIcons, initMusicInfo } =
	useMusicPlayer();

/**
 * Lo que dice el globo: qué suena, de quién, de qué disco y por dónde va.
 */
const summary = computed(() => {
	const info = musicInfo.value;
	if (!info.title) return t('components.TrayMusicControl.nothingPlaying');

	const lines = [info.title];
	if (info.artist) lines.push(info.artist);
	if (info.album) lines.push(info.album);
	if (info.length > 0) {
		lines.push(`${formatDuration(position.value)} / ${formatDuration(info.length)}`);
	}
	return lines.join('\n');
});

/** El nombre del botón: qué hace y qué suena, para quien no ve la portada. */
const accessibleName = computed(() => {
	const open = t('components.TrayMusicControl.openPlayer');
	const info = musicInfo.value;
	if (!info.title) return open;
	return [open, info.artist ? `${info.title} — ${info.artist}` : info.title].join(': ');
});

/** Si suena, está en pausa o no hay nada: el disco gira, se congela o se queda quieto. */
const state = computed(() => playbackStateOf(musicInfo.value.status));

const showTitle = computed(() => !vertical.value && Boolean(musicInfo.value.title));

/**
 * «01:42 / 04:19», con los minutos en dos cifras como en el video de
 * referencia; con horas queda como viene («1:02:03»). Sin largo conocido (una
 * radio en vivo), nada.
 */
const clock = (micros: number) => formatDuration(micros).padStart(5, '0');
const timing = computed(() => {
	const info = musicInfo.value;
	if (!showTitle.value || info.length <= 0) return '';
	return `${clock(position.value)} / ${clock(info.length)}`;
});

const opener = ref<unknown>(null);
const { isOpen } = useOpenApplet('music');

async function openPlayer(): Promise<void> {
	try {
		await toggleApplet('music', opener.value);
	} catch (error) {
		logError('[TrayMusicControl] no se pudo abrir el reproductor:', error);
	}
}

onMounted(async () => {
	await initIcons();
	await initMusicInfo();
});
</script>

<template>
  <!-- La píldora es un botón entero: se apunta entera, se realza mientras el
       reproductor está abierto y es el ancla de dónde se despliega. Adentro,
       la portada (`SpinningCover`, que se congela en pausa en vez de volver a
       cero y se queda quieta con menos movimiento) con el aro de avance, el
       título cortado y la posición. -->
  <PanelPill
    ref="opener"
    :label="showTitle ? musicInfo.title : ''"
    :caption="timing"
    :expanded="isOpen"
    :title="summary"
    :accessible-label="accessibleName"
    :orientation="vertical ? 'vertical' : 'horizontal'"
    class="max-w-56"
    data-music-pill
    @click="openPlayer"
  >
    <template #leading>
      <SpinningCover
        class="size-6 shrink-0"
        :src="imgSrc"
        :alt="musicInfo.title"
        :state="state"
        :progress="musicInfo.length > 0 ? progress * 100 : null"
        :progress-label="t('components.TrayMusicControl.progress')"
        @error="onImgError"
      />
    </template>
  </PanelPill>
</template>
