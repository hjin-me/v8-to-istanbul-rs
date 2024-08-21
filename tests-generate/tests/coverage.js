const fs = require('node:fs/promises')

/**
 * @param {import('@playwright/test').test} testSets
 */
function coverage(testSets) {
    let unsupported = false
    testSets.beforeEach(async ({page}) => {
        if (unsupported) {
            return
        }
        try {
            await page.coverage.startJSCoverage()
        } catch {
            unsupported = true
        }
    })

    testSets.afterEach(async ({page}, testInfo) => {
        if (unsupported) {
            return
        }
        const coverage = await page.coverage.stopJSCoverage()
        const outputCoverageJson = testInfo.outputPath(`v8-coverage.json`)
        await fs.writeFile(outputCoverageJson, JSON.stringify(coverage))
    })
}

module.exports = coverage
