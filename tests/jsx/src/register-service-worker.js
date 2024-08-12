/* eslint-disable import/unambiguous -- 无 import */
if (process.env.NODE_ENV === 'production') {
  const sw = navigator.serviceWorker
  const swController = sw.controller
  if (swController) {
    sw.addEventListener('message', event => {
      if (event.data?.type === 'VERSION') {
        sw.register(`/sw.js?v=${event.data.version}`, { scope: '/' })
      }
    })
    // eslint-disable-next-line unicorn/require-post-message-target-origin -- 无需判断
    swController.postMessage({ type: 'GET_VERSION' })
  } else {
    sw.register('/sw.js', { scope: '/' })
  }
}
/* eslint-enable import/unambiguous -- 恢复 eslint 规则 */
