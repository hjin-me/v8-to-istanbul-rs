import { useState } from 'react'
import { observer } from 'mobx-react-lite'
import { TodoListModel } from './model'

const Todo = observer(({ todo }) => (
  <li>
    <input
      type="checkbox"
      checked={todo.done}
      onChange={() => {
        todo.toggle()
      }}
    />
    {todo.title}
  </li>
))

const TodoList = observer(() => {
  const [todoList] = useState(() => new TodoListModel())
  return (
    <>
      <form
        onSubmit={e => {
          e.preventDefault()
          todoList.addTodo(e.target.todo.value)
        }}
      >
        New Todo: <input type="text" name="todo" /> <button type="submit">Add</button>
      </form>
      <ul>
        {todoList.todos.map(todo => (
          <Todo todo={todo} key={todo.id} />
        ))}
      </ul>
      Tasks left: {todoList.unfinishedTodoCount}
    </>
  )
})

export default TodoList
