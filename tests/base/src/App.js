import initialContextHolder, { App as AntdApp } from 'antd'
import { ConfigProvider as ConfigProviderV4 } from '@fe/track-antd/lib/es-antd'

import { Provider, connect } from 'react-redux'
import TrackConfigProvider from 'shared_libs_antd_5/@/track/ConfigProvider'
import qs from 'qs'
import { setUserInfo, setFeatureFlag } from '@/redux/actions'
import '@fe/track-antd/lib/clickHandle'
import { setConfig } from '@fe/track-antd/lib'
import AntdLowVersionProvider from 'antd-low-version-provider'
import { SWRConfig } from 'swr'
import BrowserCheck from './components/BrowserCheck'
import store from './redux/store'
import Routes from './route/router'
import ThemeConfigProvider from './theme'

ConfigProviderV4.config({
  theme: {
    primaryColor: 'var(--color-focus-0)',
  },
})

setConfig('antd', {
  dialog: {
    open: 'view',
    close: 'view',
  },
  Form: {
    onValuesChange: {
      disabled: false,
    },
  },
})

const swrOptions = {
  revalidateOnFocus: false,
  errorRetryCount: 0,
}

function App() {
  return (
    <SWRConfig value={swrOptions}>
      <AntdLowVersionProvider>
        <TrackConfigProvider
          defaultConfig={{
            instance: '',
            antd: {
              popup: {
                openChange: 'expose',
              },
              Form: {
                onValuesChange: {
                  disabled: false,
                },
              },
              Tooltip: false,
              Popconfirm: false,
              Popover: false,
            },
          }}
        >
          <Provider store={store}>
            <ThemeConfigProvider>
              <AntdApp>
                <BrowserCheck project="fe-data-workshop" />
                <InitApp />
              </AntdApp>
            </ThemeConfigProvider>
          </Provider>
        </TrackConfigProvider>
      </AntdLowVersionProvider>
    </SWRConfig>
  )
}

const removeNonISOAndChineseCharacters = inputString =>
  // eslint-disable-next-line no-control-regex -- 此处需要覆盖到所有特殊字符
  inputString.replaceAll(/[^\u0020-\u007E\u4E00-\u9FA5\uFF1A\u3010\u3011:._\-&=]/gu, '')

const hasNonISOAndChineseCharacters = inputString =>
  // eslint-disable-next-line no-control-regex -- 此处需要覆盖到除Unicode和中文字符外的所有特殊字符
  /[^\u0020-\u007E\u4E00-\u9FA5\uFF1A\u3010\u3011:._\-&=]/u.test(inputString)

const InitApp = connect(null, { setUserInfo, setFeatureFlag })(({ setFeatureFlag }) => {
  initialContextHolder()
  const searchString = window.location.search
  const { feature = [] } = qs.parse(searchString.slice(1, -1))
  if (feature.length > 0) {
    const featureFlag = {}
    const array = Array.isArray(feature) ? feature : [feature]
    for (const element of array) {
      featureFlag[element] = true
    }
    setFeatureFlag(featureFlag)
  }

  // 此处逻辑是为了处理url中含有非ISO-8859-1标准字符会导致request报错的问题
  const currentUrl = decodeURI(window.location.href)
  if (hasNonISOAndChineseCharacters(currentUrl)) {
    const sanitizedUrl = encodeURI(removeNonISOAndChineseCharacters(currentUrl))
    window.location.replace(sanitizedUrl)
  }

  return <Routes />
})
export default App
