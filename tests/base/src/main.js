;(() => {
  const rand = Math.random()

  function f1() {
    console.log('f1')
  }

  if (rand > 0.5) {
    f1()
  }

  const f3 = function () {
    console.log('f3')
  }
  if (rand < 0.5) {
    const f2 = function () {
      console.log('f2')
    }
    f2()
    f3()
  }
  rand > 0.7 && f3()
  f3()
  f3()
  f3()

  setTimeout(() => {
    console.log('timeout')
  }, 9_999_999)
})()
