<script setup lang="ts">
/** biome-ignore-all lint/correctness/noUnusedImports: <Use in template> */
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */
/**
 * El chat de Lince (`vasak-assistant`).
 *
 * Todo lo que llega del servidor o del modelo —mensajes, resumen y campos de la
 * acción propuesta, texto del resultado— se dibuja como texto (`{{ }}`), nunca
 * como HTML: el modelo puede ser inducido a escribir cualquier cosa.
 *
 * No se cierra al perder el foco (la superficie de Rust no lo hace); se cierra
 * con Escape o con el botón, vía `hideAssistant`, nunca con el conmutador.
 */
import { useI18n } from '@vasakgroup/tauri-plugin-i18n';
import {
	ActionButton,
	AlertMessage,
	Chip,
	EmptyState,
	LoadingState,
	TextInput,
} from '@vasakgroup/vue-libvasak';
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue';
import {
	type AssistantMessage,
	type AssistantPendingAction,
	type AssistantStatus,
	assistantCancelAction,
	assistantChatTurn,
	assistantConfirmAction,
	assistantHistory,
	assistantNewConversation,
	assistantStatus,
	hideAssistant,
} from '@/services/assistant.service';
import { surfaceIsShown } from '@/services/window.service';
import { useBoxSize } from '@/tools/composables/useBoxSize';
import { useSharedEvent } from '@/tools/event.bus';

const { t } = useI18n();

const STORAGE_KEY = 'vasak.assistant.conversation';

const root = ref<HTMLElement | null>(null);
const list = ref<HTMLElement | null>(null);
const { width } = useBoxSize(root);
/** Angosto: el chip del modelo pasa a su propia línea. */
const narrow = computed(() => width.value > 0 && width.value < 420);

const messages = ref<AssistantMessage[]>([]);
const status = ref<AssistantStatus | null>(null);
const pending = ref<AssistantPendingAction | null>(null);
const draft = ref('');
const loading = ref(true);
const thinking = ref(false);
const acting = ref(false);
const errorMessage = ref('');
let conversationId = '';

const visibleMessages = computed(() => messages.value.filter((m) => m.role !== 'system'));
const busy = computed(() => thinking.value || acting.value || loading.value);
const canSend = computed(() => !busy.value && pending.value === null && draft.value.trim() !== '');
/** Las respuestas salen de la máquina hacia otro host. */
const leavesBox = computed(() => status.value !== null && !status.value.model.local);

const readStoredId = (): string => {
	try {
		return localStorage.getItem(STORAGE_KEY) ?? '';
	} catch {
		return '';
	}
};
const storeId = (id: string) => {
	try {
		localStorage.setItem(STORAGE_KEY, id);
	} catch {
		/* sin almacenamiento: la conversación no sobrevive a la recarga */
	}
};

const describe = (error: unknown): string => (typeof error === 'string' ? error : String(error));

const scrollToEnd = async () => {
	await nextTick();
	if (list.value) list.value.scrollTop = list.value.scrollHeight;
};

const push = (role: AssistantMessage['role'], content: string) => {
	messages.value.push({ role, content });
	scrollToEnd();
};

async function load(): Promise<void> {
	loading.value = true;
	errorMessage.value = '';
	try {
		status.value = await assistantStatus();
	} catch (error) {
		status.value = null;
		errorMessage.value = describe(error);
	}
	try {
		conversationId = conversationId || readStoredId();
		if (conversationId) {
			try {
				messages.value = await assistantHistory(conversationId);
			} catch {
				// El id guardado ya no existe en el servicio: se empieza otra.
				conversationId = '';
			}
		}
		if (!conversationId) {
			conversationId = await assistantNewConversation();
			storeId(conversationId);
			messages.value = [];
		}
	} catch (error) {
		errorMessage.value = describe(error);
	} finally {
		loading.value = false;
		scrollToEnd();
	}
}

async function send(): Promise<void> {
	if (!canSend.value) return;
	const content = draft.value.trim();
	draft.value = '';
	errorMessage.value = '';
	push('user', content);
	thinking.value = true;
	try {
		const reply = await assistantChatTurn(conversationId, content);
		if (reply.text) push('assistant', reply.text);
		pending.value = reply.pending[0] ?? null;
	} catch (error) {
		errorMessage.value = describe(error);
	} finally {
		thinking.value = false;
	}
}

async function confirm(): Promise<void> {
	if (!pending.value || acting.value) return;
	acting.value = true;
	errorMessage.value = '';
	try {
		push('assistant', await assistantConfirmAction(pending.value.id));
		pending.value = null;
	} catch (error) {
		errorMessage.value = describe(error);
	} finally {
		acting.value = false;
	}
}

async function cancel(): Promise<void> {
	if (!pending.value || acting.value) return;
	acting.value = true;
	errorMessage.value = '';
	try {
		await assistantCancelAction(pending.value.id);
		pending.value = null;
	} catch (error) {
		errorMessage.value = describe(error);
	} finally {
		acting.value = false;
	}
}

const close = () => {
	hideAssistant().catch(() => {
		/* ya cerrada */
	});
};

const onKeydown = (event: KeyboardEvent) => {
	if (event.key === 'Escape') close();
};

const onInputKeydown = (event: KeyboardEvent) => {
	if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) {
		event.preventDefault();
		send();
	}
};

// Se vuelve a mostrar sin recargar: se refresca el estado y el historial.
useSharedEvent('window-shown', load);

onMounted(async () => {
	document.addEventListener('keydown', onKeydown);
	// Recarga de WebKit: el `window-shown` se perdió, se consulta el estado.
	if (await surfaceIsShown('assistant')) await load();
	else loading.value = false;
});

onBeforeUnmount(() => document.removeEventListener('keydown', onKeydown));
</script>

<template>
  <main
    ref="root"
    class="bg-ui-shell window-border rounded-corner-m flex h-screen w-screen min-w-0 flex-col gap-3 overflow-hidden p-4 text-tx-main"
  >
    <header class="flex shrink-0 flex-wrap items-center gap-2">
      <h1 class="min-w-0 flex-1 truncate font-semibold">{{ t('views.assistant.title') }}</h1>
      <Chip
        v-if="status && !narrow"
        :label="leavesBox ? status.model.host : t('views.assistant.local')"
        :caption="status.model.name"
        :icon="leavesBox ? 'network-wireless' : 'computer'"
        :title="leavesBox ? t('views.assistant.leavesBox').replace('{0}', status.model.host) : t('views.assistant.local')"
      />
      <ActionButton
        label=""
        icon="window-close"
        :icon-alt="t('views.assistant.close')"
        :title="t('views.assistant.close')"
        variant="ghost"
        @click="close"
      />
    </header>

    <!-- «Sale de la caja»: con un modelo remoto lo que se escribe viaja a ese host. -->
    <AlertMessage v-if="leavesBox" tone="warning" class="shrink-0">
      {{ t('views.assistant.leavesBox').replace('{0}', status?.model.host ?? '') }}
    </AlertMessage>
    <AlertMessage v-if="status && !status.backend_available" tone="error" class="shrink-0">
      {{ t('views.assistant.unavailable') }}{{ status.reason ? `: ${status.reason}` : '' }}
    </AlertMessage>

    <LoadingState v-if="loading" class="flex-1" :label="t('views.assistant.loading')" />

    <template v-else>
      <ul
        ref="list"
        class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto"
        aria-live="polite"
        data-messages
      >
        <li v-if="visibleMessages.length === 0 && !thinking">
          <EmptyState :title="t('views.assistant.empty')" icon="" size="sm" />
        </li>
        <li
          v-for="(message, index) in visibleMessages"
          :key="index"
          :data-role="message.role"
          :class="[
            'max-w-[85%] min-w-0 whitespace-pre-wrap break-words rounded-corner-m px-3 py-2',
            message.role === 'user'
              ? 'self-end bg-primary text-tx-on-primary'
              : 'self-start bg-ui-surface/70 border border-ui-line text-tx-main',
          ]"
        >{{ message.content }}</li>
        <li v-if="thinking" class="self-start text-tx-muted" role="status" data-thinking>
          {{ t('views.assistant.thinking') }}
        </li>
      </ul>

      <AlertMessage v-if="errorMessage" tone="error" class="shrink-0">{{ errorMessage }}</AlertMessage>

      <!-- La acción que el asistente propone: nada se ejecuta sin este botón. -->
      <section
        v-if="pending"
        class="bg-ui-surface/70 border border-ui-line rounded-corner-m flex shrink-0 flex-col gap-2 p-3"
        :aria-label="t('views.assistant.confirmTitle')"
        data-confirmation
      >
        <p class="font-semibold break-words">{{ pending.summary }}</p>
        <dl v-if="pending.kind === 'send_mail'" class="flex min-w-0 flex-col gap-1 text-label-xs">
          <div class="flex gap-2"><dt class="text-tx-muted">{{ t('views.assistant.mailTo') }}</dt><dd class="min-w-0 break-words">{{ pending.to }}</dd></div>
          <div class="flex gap-2"><dt class="text-tx-muted">{{ t('views.assistant.mailSubject') }}</dt><dd class="min-w-0 break-words">{{ pending.subject }}</dd></div>
          <p class="max-h-24 overflow-y-auto whitespace-pre-wrap break-words">{{ pending.body_preview }}</p>
        </dl>
        <div class="flex flex-wrap justify-end gap-2">
          <ActionButton :label="t('views.assistant.cancel')" variant="secondary" :disabled="acting" data-cancel @click="cancel" />
          <ActionButton :label="t('views.assistant.confirm')" :disabled="acting" data-confirm @click="confirm" />
        </div>
      </section>

      <div class="flex shrink-0 items-center gap-2">
        <TextInput
          v-model="draft"
          class="min-w-0 flex-1"
          :aria-label="t('views.assistant.placeholder')"
          :placeholder="t('views.assistant.placeholder')"
          :disabled="busy || pending !== null"
          autocomplete="off"
          @keydown="onInputKeydown"
        />
        <ActionButton :label="t('views.assistant.send')" :disabled="!canSend" data-send @click="send" />
      </div>
    </template>
  </main>
</template>
