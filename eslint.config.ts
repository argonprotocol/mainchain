import prettierConfig from 'eslint-config-prettier';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  tseslint.configs.recommendedTypeChecked,
  {
    ignores: [
      '**/node_modules/**',
      '**/*.*js',
      '**/target',
      '**/.yarn',
      '**/*.d.*ts',
      '**/*.config.ts',
    ],
  },
  {
    rules: {
      '@typescript-eslint/prefer-promise-reject-errors': 'off',
      '@typescript-eslint/no-non-null-assertion': 'off',
      '@typescript-eslint/no-explicit-any': 'off',
      '@typescript-eslint/no-unsafe-assignment': 'off',
      '@typescript-eslint/no-unsafe-argument': 'off',
      '@typescript-eslint/no-unused-vars': 'off',
      '@typescript-eslint/require-await': 'off',
      '@typescript-eslint/explicit-function-return-type': 'off',
      '@typescript-eslint/explicit-module-boundary-types': 'off',
      '@typescript-eslint/restrict-template-expressions': [
        'warn',
        { allowNumber: true, allowBoolean: true, allowNullish: true },
      ],
      '@typescript-eslint/no-unsafe-member-access': 'off',
      '@typescript-eslint/ban-ts-comment': [
        'warn',
        { 'ts-expect-error': 'allow-with-description' },
      ],
      '@typescript-eslint/no-misused-promises': [
        'error',
        { checksVoidReturn: { arguments: false } },
      ],
      '@typescript-eslint/no-floating-promises': ['error', { ignoreIIFE: true, ignoreVoid: true }],
      '@typescript-eslint/no-inferrable-types': 'off',
      '@typescript-eslint/consistent-type-assertions': [
        'warn',
        { assertionStyle: 'as', objectLiteralTypeAssertions: 'allow' },
      ],
      '@typescript-eslint/no-empty-function': [
        'warn',
        { allow: ['constructors', 'private-constructors'] },
      ],
      '@typescript-eslint/no-var-requires': 'off',
      // keep the TS version, disable the base just in case
      'no-use-before-define': 'off',
    },
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },
  // Looser in tests and scripts
  {
    files: ['**/*.test.ts', '**/__test__/**', 'scripts/**'],
    rules: {
      '@typescript-eslint/no-unsafe-assignment': 'off',
      '@typescript-eslint/restrict-template-expressions': 'off',
      '@typescript-eslint/ban-ts-comment': 'off',
      '@typescript-eslint/no-explicit-any': 'off',
      '@typescript-eslint/no-empty-function': 'off',
      '@typescript-eslint/no-non-null-assertion': 'off',
      '@typescript-eslint/no-floating-promises': 'off',
    },
  },
  {
    files: ['**/*.d.ts'],
    rules: {
      '@typescript-eslint/no-unused-vars': 'off',
      '@typescript-eslint/no-namespace': 'off',
      '@typescript-eslint/consistent-type-definitions': 'off',
      '@typescript-eslint/no-empty-interface': 'off',
      '@typescript-eslint/no-explicit-any': 'off',
      '@typescript-eslint/no-empty-object-type': 'off',
    },
  },
  prettierConfig,
);
