<script lang="ts" setup>
/** biome-ignore-all lint/correctness/noUnusedVariables: <Use in template> */

import { invoke } from '@tauri-apps/api/core';
import { getIconSource } from '@vasakgroup/plugin-vicons';
import { computed, onMounted, ref } from 'vue';
import { logError } from '@/utils/logger';

interface Shortcut {
	id: string;
	name: string;
	description: string;
	keys: string;
	category: 'system' | 'vasak' | 'custom';
	editable: boolean;
}

interface ConflictInfo {
	has_conflict: boolean;
	conflict_with?: string;
	message: string;
}

const shortcuts = ref<Shortcut[]>([]);
const editingId = ref<string | null>(null);
const editingKeys = ref('');
const error = ref('');
const successMessage = ref('');
const loading = ref(true);
const testingId = ref<string | null>(null);

// Iconos
const keyboardIconSrc = ref('');
const saveIconSrc = ref('');
const cancelIconSrc = ref('');
const trashIconSrc = ref('');
const playIconSrc = ref('');

// Conflicto actual mientras edita
const currentConflict = ref<ConflictInfo | null>(null);

onMounted(async () => {
	// Cargar iconos
	keyboardIconSrc.value = await getIconSource('keyboard');
	saveIconSrc.value = await getIconSource('check');
	cancelIconSrc.value = await getIconSource('x');
	trashIconSrc.value = await getIconSource('trash2');
	playIconSrc.value = await getIconSource('play');

	// Cargar atajos
	try {
		const data = await invoke<Shortcut[]>('get_shortcuts');
		shortcuts.value = data;
	} catch (err) {
		logError('Error loading shortcuts:', err);
		error.value = 'Error al cargar los atajos';
	} finally {
		loading.value = false;
	}
});

const startEdit = (shortcut: Shortcut) => {
	if (!shortcut.editable) return;
	editingId.value = shortcut.id;
	editingKeys.value = shortcut.keys;
	error.value = '';
	successMessage.value = '';
	currentConflict.value = null;
};

const cancelEdit = () => {
	editingId.value = null;
	editingKeys.value = '';
	error.value = '';
	currentConflict.value = null;
};

const checkConflicts = async () => {
	if (!editingKeys.value.trim()) {
		currentConflict.value = null;
		return;
	}

	try {
		const conflict = await invoke<ConflictInfo>('check_shortcut_conflicts', {
			keys: editingKeys.value,
			exclude_id: editingId.value,
		});
		currentConflict.value = conflict;
	} catch (err) {
		logError('Error verificando conflictos de atajos:', err);
	}
};

const saveShortcut = async () => {
	if (!editingId.value || !editingKeys.value.trim()) {
		error.value = 'Las teclas no pueden estar vacías';
		return;
	}

	// Validar formato (Ctrl+Key, Alt+Key, Super+Key, etc.)
	const validPattern = /^(Ctrl|Alt|Super|Shift)(\+[A-Za-z0-9])?$/;
	if (!validPattern.test(editingKeys.value)) {
		error.value = 'Formato inválido. Usa: Ctrl+A, Alt+F, Super+S, etc.';
		return;
	}

	// Verificar conflictos
	if (currentConflict.value?.has_conflict) {
		error.value = `Conflicto: ${currentConflict.value.message}`;
		return;
	}

	try {
		await invoke('update_shortcut', {
			id: editingId.value,
			keys: editingKeys.value,
		});

		const shortcut = shortcuts.value.find((s) => s.id === editingId.value);
		if (shortcut) {
			shortcut.keys = editingKeys.value;
		}

		editingId.value = null;
		editingKeys.value = '';
		error.value = '';
		successMessage.value = '✅ Atajo guardado exitosamente';
		setTimeout(() => {
			successMessage.value = '';
		}, 3000);
	} catch (err) {
		error.value = `Error al guardar: ${err}`;
	}
};

const testShortcut = async (shortcutId: string) => {
	testingId.value = shortcutId;
	try {
		await invoke('execute_shortcut', { shortcutId });
		successMessage.value = '✅ Atajo ejecutado';
		setTimeout(() => {
			successMessage.value = '';
		}, 2000);
	} catch (err) {
		error.value = `Error al ejecutar: ${err}`;
		setTimeout(() => {
			error.value = '';
		}, 3000);
	} finally {
		testingId.value = null;
	}
};

const deleteShortcut = async (id: string) => {
	if (!confirm('¿Estás seguro de que deseas eliminar este atajo personalizado?')) return;

	try {
		await invoke('delete_shortcut', { id });
		shortcuts.value = shortcuts.value.filter((s) => s.id !== id);
		successMessage.value = '✅ Atajo eliminado';
		setTimeout(() => {
			successMessage.value = '';
		}, 2000);
	} catch (err) {
		error.value = `Error al eliminar: ${err}`;
	}
};

const addCustomShortcut = async () => {
	const name = prompt('Nombre del atajo:');
	if (!name) return;

	const description = prompt('Descripción (opcional):');
	const keys = prompt('Teclas (ej: Ctrl+K):');
	if (!keys) return;

	const command = prompt('Comando a ejecutar:');
	if (!command) return;

	try {
		const newShortcut = await invoke<Shortcut>('add_custom_shortcut', {
			name,
			description: description || '',
			keys,
			command,
		});
		shortcuts.value.push(newShortcut);
		successMessage.value = '✅ Atajo personalizado creado';
		setTimeout(() => {
			successMessage.value = '';
		}, 2000);
	} catch (err) {
		error.value = `Error al crear atajo: ${err}`;
	}
};

const getCategoryLabel = (category: string): string => {
	const labels: Record<string, string> = {
		system: 'Sistema',
		vasak: 'VasakOS',
		custom: 'Personalizado',
	};
	return labels[category] || category;
};

const getCategoryColor = (category: string): string => {
	const colors: Record<string, string> = {
		system: 'bg-blue-500/10 text-blue-600',
		vasak: 'bg-purple-500/10 text-purple-600',
		custom: 'bg-green-500/10 text-green-600',
	};
	return colors[category] || 'bg-gray-500/10 text-gray-600';
};

const hasConflict = computed(() => currentConflict.value?.has_conflict ?? false);
const showConflictWarning = computed(() => {
	return editingId.value && currentConflict.value?.has_conflict;
});
</script>

<template>
  <ConfigAppLayout>
    <div class="w-full h-full flex flex-col">
      <!-- Header -->
      <div class="flex items-center gap-3 mb-6">
        <img :src="keyboardIconSrc" alt="Shortcuts" class="w-6 h-6" />
        <h1 class="text-2xl font-bold text-vsk-text">Atajos de Teclado</h1>
      </div>

      <!-- Success Message -->
      <div v-if="successMessage" class="mb-4 p-3 rounded-vsk bg-green-500/10 border border-green-500/30 text-green-600 text-sm animate-pulse">
        {{ successMessage }}
      </div>

      <!-- Error Message -->
      <div v-if="error" class="mb-4 p-3 rounded-vsk bg-red-500/10 border border-red-500/30 text-red-600 text-sm">
        {{ error }}
      </div>

      <!-- Loading -->
      <div v-if="loading" class="text-center py-8 text-vsk-text/50">
        Cargando atajos...
      </div>

      <!-- Content -->
      <div v-else class="flex-1 flex flex-col gap-4">
        <!-- Add Custom Button -->
        <button
          @click="addCustomShortcut"
          class="self-start px-4 py-2 rounded-vsk bg-vsk-primary/20 border border-vsk-primary/50 text-vsk-primary font-semibold hover:bg-vsk-primary/30 transition-colors"
        >
          + Agregar Atajo Personalizado
        </button>

        <!-- Shortcuts List -->
        <div class="flex-1 overflow-y-auto space-y-3">
          <div
            v-for="shortcut in shortcuts"
            :key="shortcut.id"
            class="p-4 rounded-vsk background border border-vsk-primary/10 hover:border-vsk-primary/20 transition-all"
            :class="{ 'border-red-500/50 bg-red-500/5': showConflictWarning && shortcut.id === editingId }"
          >
            <!-- Header -->
            <div class="flex items-start justify-between gap-4">
              <div class="flex-1 min-w-0">
                <h3 class="text-lg font-bold text-vsk-text">{{ shortcut.name }}</h3>
                <p v-if="shortcut.description" class="text-sm text-vsk-text/60 mt-1">
                  {{ shortcut.description }}
                </p>
              </div>

              <!-- Category Badge -->
              <span :class="[
                'px-2 py-1 rounded-full text-xs font-semibold whitespace-nowrap shrink-0',
                getCategoryColor(shortcut.category)
              ]">
                {{ getCategoryLabel(shortcut.category) }}
              </span>
            </div>

            <!-- Conflict Warning -->
            <div v-if="showConflictWarning && shortcut.id === editingId && currentConflict?.has_conflict" class="mt-3 p-2 rounded-vsk bg-red-500/20 border border-red-500/30 text-red-600 text-xs">
              ⚠️ {{ currentConflict.message }}
            </div>

            <!-- Keys Row -->
            <div class="mt-4 flex items-center gap-3">
              <div v-if="editingId !== shortcut.id" class="flex items-center gap-3 flex-1">
                <div class="px-3 py-2 rounded-vsk bg-vsk-primary/15 border border-vsk-primary/30 font-mono text-vsk-primary">
                  {{ shortcut.keys }}
                </div>
              </div>

              <!-- Edit Input -->
              <div v-else class="flex items-center gap-2 flex-1">
                <input
                  v-model="editingKeys"
                  @input="checkConflicts"
                  type="text"
                  placeholder="Ej: Ctrl+K"
                  class="flex-1 px-3 py-2 rounded-vsk bg-vsk-primary/5 border border-vsk-primary/30 text-vsk-text placeholder:text-vsk-text/30 focus:outline-none focus:border-vsk-primary/60 font-mono"
                  :class="{ 'border-red-500/50 bg-red-500/5': hasConflict }"
                />
              </div>

              <!-- Action Buttons -->
              <div class="flex gap-2">
                <ActionButton
                  v-if="editingId === shortcut.id"
                  label=""
                  :iconSrc="saveIconSrc"
                  iconAlt="Guardar"
                  size="sm"
                  variant="secondary"
                  :disabled="hasConflict"
                  customClass="bg-transparent text-green-600 hover:bg-green-500/20 border border-transparent"
                  @click="saveShortcut"
                />

                <ActionButton
                  v-if="editingId === shortcut.id"
                  label=""
                  :iconSrc="cancelIconSrc"
                  iconAlt="Cancelar"
                  size="sm"
                  variant="secondary"
                  customClass="bg-transparent text-red-600 hover:bg-red-500/20 border border-transparent"
                  @click="cancelEdit"
                />

                <ActionButton
                  v-if="editingId !== shortcut.id"
                  label=""
                  :iconSrc="playIconSrc"
                  iconAlt="Probar"
                  size="sm"
                  variant="secondary"
                  :disabled="testingId === shortcut.id"
                  customClass="bg-transparent text-blue-600 hover:bg-blue-500/20 border border-transparent"
                  @click="testShortcut(shortcut.id)"
                />

                <button
                  v-if="editingId !== shortcut.id && shortcut.editable"
                  @click="startEdit(shortcut)"
                  class="px-3 py-2 rounded-vsk bg-vsk-primary/20 text-vsk-primary text-xs font-semibold hover:bg-vsk-primary/30 transition-colors"
                >
                  Editar
                </button>

                <ActionButton
                  v-if="editingId !== shortcut.id && shortcut.category === 'custom'"
                  label=""
                  :iconSrc="trashIconSrc"
                  iconAlt="Eliminar"
                  size="sm"
                  variant="secondary"
                  customClass="bg-transparent text-red-600 hover:bg-red-500/20 border border-transparent"
                  @click="deleteShortcut(shortcut.id)"
                />
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Help Text -->
      <div class="mt-6 p-3 rounded-vsk bg-vsk-primary/5 border border-vsk-primary/20 text-vsk-text/60 text-xs">
        <p class="font-semibold mb-2">Formato de atajos:</p>
        <ul class="space-y-1 ml-2">
          <li>• <code class="font-mono">Ctrl+K</code> - Control + una tecla</li>
          <li>• <code class="font-mono">Alt+Space</code> - Alt + una tecla</li>
          <li>• <code class="font-mono">Super+S</code> - Tecla Windows/Super + una tecla</li>
          <li>• <code class="font-mono">Shift+O</code> - Shift + una tecla</li>
        </ul>
        <p class="font-semibold mt-3 mb-2">Consejos:</p>
        <ul class="space-y-1 ml-2">
          <li>• Usa el botón ▶️ para probar el atajo antes de guardarlo</li>
          <li>• Los conflictos se detectan automáticamente</li>
          <li>• Los atajos del sistema no se pueden editar, solo los personalizados</li>
        </ul>
      </div>
    </div>
  </ConfigAppLayout>
</template>

<style scoped>
code {
  background-color: rgba(var(--primary-color), 0.1);
  padding: 0.125rem 0.25rem;
  border-radius: 0.25rem;
}

@keyframes pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.7;
  }
}

.animate-pulse {
  animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}
</style>
