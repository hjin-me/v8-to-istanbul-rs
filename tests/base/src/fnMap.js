const proto = window.location.href.includes('https') ? 'https' : 'http'

function f6() {
  {
    const a = Date.now()
    console.log(`this is used block 1 ${a}`)
  }
  {
    const a = Date.now()
    console.log(`this is used block 2 ${a}`)
  }
}

function f7() {
  {
    const a = Date.now()
    console.log(`this is unused block 1 ${a}`)
  }
  {
    const a = Date.now()
    console.log(`this is unused block 2 ${a}`)
  }
}

if (proto === 'http') {
  f8()
} else {
  f9()
}

function f8() {
  if (proto === 'http') {
    f6()
  } else {
    f7()
  }
}

function f9() {
  if (proto === 'http') {
    f6()
  } else {
    f7()
  }
}

function f10() {
  const f11 = () => {
    console.log('used')
  }
  const f12 = () => {
    console.log('used')
  }

  function f13() {
    console.log('used')
  }

  function f14() {
    console.log('used')
  }

  if (proto === 'http') {
    f11()
    f13()
  } else {
    f12()
    f14()
  }
}

f10()

const f1 = () => {
  console.log('used')
}
const f2 = function () {
  console.log('used')
}
const f3 = () => {
  console.log('not used')
}
const f4 = function () {
  console.log('not used')
}
if (proto === 'http') {
  f1()
  f2()
} else {
  f3()
  f4()

  const f5 = () => {
    console.log('f5')
  }

  f5()
}

function f15() {
  if (proto === 'http') {
    const f16 = () => {
      console.log('f16')
    }
    f16()
  } else {
    const f17 = () => {
      console.log('f17')
    }
    f17()
  }
}

f15()
