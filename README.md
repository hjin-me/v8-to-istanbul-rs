# V8 Coverage to Istanbul Coverage

## v8 coverage + source map => raw code coverage

```bash
v8-to-istanbul convert --pattern "test-results/**/v8-coverage.json" --filters "{xxx.min.js}" --output ./ --merge --use-local
```

### difference with https://github.com/istanbuljs/v8-to-istanbul

- coverage for all code in source map (not only code in v8 coverage)
- better performance (perhaps?)


### TODO

[ ] Functions
[ ] Branch