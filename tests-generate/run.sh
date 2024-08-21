npm run build
docker run --rm -d -v `pwd`:/usr/share/nginx/html -p 8080:80 nginx
npx playwright test --project dev
node ../cli.js -a convert -i ./test-results -s ./src -d .nyc_output/ -p "http://127.0.0.1"
npx nyc report --reporter=lcov
