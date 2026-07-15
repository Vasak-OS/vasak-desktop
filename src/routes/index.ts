import { createRouter, createWebHashHistory } from 'vue-router';
import { ipcBatch } from '@/tools/ipc.batch';

const routes = [
	{ path: '/desktop', component: () => import('@/views/DesktopView.vue') },
	{ path: '/panel', component: () => import('@/views/PanelView.vue') },
	{ path: '/menu', component: () => import('@/views/MenuView.vue') },
	{
		path: '/control_center',
		component: () => import('@/views/ControlCenterView.vue'),
	},
	{
		path: '/applets',
		children: [
			{
				path: 'bluetooth',
				component: () => import('@/views/applets/BluetoothAppletView.vue'),
			},
			{
				path: 'network',
				component: () => import('@/views/applets/NetworkAppletView.vue'),
			},
			{
				path: 'audio',
				component: () => import('@/views/applets/AudioAppletView.vue'),
			},
			{
				path: 'tray-popup',
				component: () => import('@/views/applets/TrayPopupView.vue'),
			},
		],
	},
	{
		path: '/apps',
		children: [
			{
				path: 'terminal',
				component: () => import('@/views/apps/TerminalView.vue'),
			},
			{ path: 'search', component: () => import('@/views/apps/SearchView.vue') },
			{ path: 'osd-popup', component: () => import('@/views/apps/OsdPopupView.vue') },
			{
				path: 'session-popup',
				component: () => import('@/views/apps/SessionPopupView.vue'),
			},
		],
	},
];

export const router = createRouter({
	history: createWebHashHistory(),
	routes,
});

/**
 * Prefetch data for the target view before navigation completes.
 * This fires a single batched IPC request with all commands the target view needs,
 * warming the cache so the component can render faster on mount.
 */
router.beforeEach((to, _from, next) => {
	ipcBatch.prefetch(to.path);
	next();
});
