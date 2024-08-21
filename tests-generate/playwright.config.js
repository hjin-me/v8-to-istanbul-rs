// @ts-check
const { defineConfig, devices } = require('@playwright/test')

/**
 * @see https://playwright.dev/docs/test-configuration
 */
module.exports = defineConfig({
  timeout: 60_000,
  testDir: './tests/',
  /* Reporter to use. See https://playwright.dev/docs/test-reporters */
  reporter: [['line']],

  /* Configure projects for major browsers */
  projects: [
    {
      name: 'dev',
      use: {
        ...devices['Desktop Chrome'],
      },
    },
  ],
})
