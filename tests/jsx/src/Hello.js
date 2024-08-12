import { useState } from 'react'
import styles from './Hello.module.less'

const Hello = () => {
  const [value, setValue] = useState('')
  return (
    <>
      <input
        onChange={e => {
          setValue(e.target.value)
        }}
      />
      <p className={styles.p}>hello {value}</p>
    </>
  )
}

export default Hello
