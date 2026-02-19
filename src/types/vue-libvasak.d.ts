/* eslint-disable @typescript-eslint/no-empty-object-type */
/** biome-ignore-all lint/complexity/noBannedTypes: <False positive> */
declare module '@vasakgroup/vue-libvasak' {
	import { DefineComponent, App } from 'vue';

	/* Layout */
	export interface WindowFrameProps {
		title?: string;
		image?: string;
		class?: string;
		[key: string]: any;
	}

	/* Sidebar */
	export interface SideBarProps {
		class?: string;
		[key: string]: any;
	}

	export interface SideButtonProps {
		title?: string;
		image?: string;
		class?: string;
		[key: string]: any;
	}

	/* Controls */
	export interface ActionButtonProps {
		label: string;
		disabled?: boolean;
		variant?: 'primary' | 'secondary' | 'danger';
		loading?: boolean;
		customClass?: string | Record<string, boolean>;
		size?: 'sm' | 'md' | 'lg';
		fullWidth?: boolean;
		iconSrc?: string;
		iconAlt?: string;
		iconRight?: boolean;
		type?: 'button' | 'submit' | 'reset';
		stopPropagation?: boolean;
		preventDefault?: boolean;
		[key: string]: any;
	}

	export interface ToggleControlProps {
		icon: string;
		alt?: string;
		tooltip?: string;
		isActive?: boolean;
		isLoading?: boolean;
		iconClass?: Record<string, boolean>;
		customClass?: Record<string, boolean>;
		[key: string]: any;
	}

	/* Layout/Config */
	export interface ConfigSectionProps {
		title: string;
		icon?: string;
		customClass?: string | Record<string, boolean>;
		[key: string]: any;
	}

	/* Cards */
	export interface DeviceCardProps {
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
		[key: string]: any;
	}

	export interface ListCardProps {
		clickable?: boolean;
		customClass?: string | Record<string, boolean>;
		[key: string]: any;
	}

	/* Forms */
	export interface FormGroupProps {
		label: string;
		htmlFor?: string;
		customClass?: string | Record<string, boolean>;
		labelClass?: string | Record<string, boolean>;
		[key: string]: any;
	}

	export interface SliderControlProps {
		icon: string;
		alt?: string;
		tooltip?: string;
		modelValue: number;
		min?: number;
		max?: number;
		showButton?: boolean;
		iconClass?: string | Record<string, boolean>;
		getPercentageClass?: (percentage: number) => string;
		[key: string]: any;
	}

	export interface SwitchToggleProps {
		isOn: boolean;
		disabled?: boolean;
		size?: 'small' | 'medium';
		activeClass?: string;
		inactiveClass?: string;
		customClass?: string;
		[key: string]: any;
	}

	/* Tray */
	export interface TrayIconButtonProps {
		icon: string;
		alt?: string;
		tooltip?: string;
		badge?: number | null;
		iconClass?: string | Record<string, boolean>;
		customClass?: string | Record<string, boolean>;
		tooltipClass?: string | Record<string, boolean>;
		showCustomTooltip?: boolean;
		customTooltipText?: string;
		[key: string]: any;
	}

	/* Component exports */
	export const WindowFrame: DefineComponent<WindowFrameProps, any, any, {}, {}, {}, {}, {}, string>;
	export const SideBar: DefineComponent<SideBarProps, any, any, {}, {}, {}, {}, {}, string>;
	export const SideButton: DefineComponent<SideButtonProps, any, any, {}, {}, {}, {}, {}, string>;

	export const ActionButton: DefineComponent<
		ActionButtonProps,
		any,
		any,
		{},
		{},
		{},
		{},
		{ click: () => void },
		string
	>;
	export const ToggleControl: DefineComponent<
		ToggleControlProps,
		any,
		any,
		{},
		{},
		{},
		{},
		{ click: () => void },
		string
	>;
	export const ConfigSection: DefineComponent<
		ConfigSectionProps,
		any,
		any,
		{},
		{},
		{},
		{},
		{},
		string
	>;

	export const DeviceCard: DefineComponent<
		DeviceCardProps,
		any,
		any,
		{},
		{},
		{},
		{},
		{ action: () => void; click: () => void },
		string
	>;
	export const ListCard: DefineComponent<
		ListCardProps,
		any,
		any,
		{},
		{},
		{},
		{},
		{ click: () => void },
		string
	>;

	export const FormGroup: DefineComponent<FormGroupProps, any, any, {}, {}, {}, {}, {}, string>;
	export const SliderControl: DefineComponent<
		SliderControlProps,
		any,
		any,
		{},
		{},
		{},
		{},
		{ 'update:modelValue': (value: number) => void; buttonClick: () => void },
		string
	>;
	export const SwitchToggle: DefineComponent<
		SwitchToggleProps,
		any,
		any,
		{},
		{},
		{},
		{},
		{ toggle: (value: boolean) => void },
		string
	>;

	export const TrayIconButton: DefineComponent<
		TrayIconButtonProps,
		any,
		any,
		{},
		{},
		{},
		{},
		{ click: () => void },
		string
	>;

	// Plugin de Vue
	export interface VueLibVasakPlugin {
		install(app: App): void;
	}

	declare const plugin: VueLibVasakPlugin;
	export default plugin;
}
