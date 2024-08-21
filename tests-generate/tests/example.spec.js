// @ts-check
const {test} = require('@playwright/test')
const coverage = require('./coverage')

coverage(test)
test('打开页面', async ({page}) => {
    await page.goto('http://127.0.0.1:3000/index.html')
    await page.waitForLoadState('networkidle')
})
