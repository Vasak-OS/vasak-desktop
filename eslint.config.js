import js from '@eslint/js';
import globals from 'globals';
import tseslint from 'typescript-eslint';
import vue from 'eslint-plugin-vue';

export default [
	js.configs.recommended,
	...tseslint.configs.recommended,
	...vue.configs['flat/essential'],
	{
		files: ['**/*.{js,mjs,cjs,ts,vue}'],
		languageOptions: {
			ecmaVersion: 'latest',
			sourceType: 'module',
			globals: {
				...globals.browser,
				...globals.node,
				...globals.es2021,
			},
			parser: tseslint.parser,
			parserOptions: {
				parser: tseslint.parser,
				extraFileExtensions: ['.vue'],
			},
		},
		rules: {
			'indent': ['error', 'tab'],
			'linebreak-style': ['error', 'unix'],
			'quotes': ['error', 'single'],
			'semi': ['error', 'always'],
			'@typescript-eslint/no-explicit-any': 'off',
		},
	},
	{
		files: ['**/*.vue'],
		languageOptions: {
			parserOptions: {
				parser: tseslint.parser,
			},
		},
	},
	{
		ignores: ['dist/', 'node_modules/', 'src-tauri/target/', '*.config.js', '*.config.ts'],
	},
];
