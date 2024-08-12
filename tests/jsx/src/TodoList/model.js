import { makeAutoObservable } from 'mobx'
import Debug from 'debug'

const debug = Debug('TodoList:model')

export class TodoModel {
  id = Math.random()
  title
  done = false

  constructor(title) {
    this.title = title
    makeAutoObservable(this)
  }

  toggle() {
    debug('toggle')
    this.done = !this.done
  }
}

export class TodoListModel {
  todos = []

  get unfinishedTodoCount() {
    return this.todos.filter(todo => !todo.done).length
  }

  constructor() {
    makeAutoObservable(this)
  }

  addTodo(title) {
    debug('addTodo', title)
    this.todos.push(new TodoModel(title))
  }
}
