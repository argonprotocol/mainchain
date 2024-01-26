/** @type {import('ts-jest').JestConfigWithTsJest} */
module.exports = {
  silent: true,
  verbose: false,
  preset: 'ts-jest',
  testEnvironment: 'node',
  testMatch: ["**/__test__/*.test.ts"],
  transform: {
    '^.+\\.ts': [
      'ts-jest',
      {
        useESM: false,
        tsconfig: './tsconfig-cjs.json',
      },
    ],
  },
};