"use strict";(globalThis.webpackChunkreact_template=globalThis.webpackChunkreact_template||[]).push([[179],{963:(e,t,s)=>{s.r(t),s(626);var o=s(982),n=s(128),r=s(142),d=s(871);let i=()=>{let[e,t]=(0,n.useState)("");return(0,d.jsxs)(d.Fragment,{children:[(0,d.jsx)("input",{onChange:e=>{t(e.target.value)}}),(0,d.jsxs)("p",{className:"Hello_p__hiSbQ",children:["hello ",e]})]})};var l=s(692),a=s(979),h=s.n(a);class u{constructor(e){h()(this,"id",Math.random()),h()(this,"title",void 0),h()(this,"done",!1),this.title=e,(0,o.makeAutoObservable)(this)}toggle(){this.done=!this.done}}class c{get unfinishedTodoCount(){return this.todos.filter(e=>!e.done).length}constructor(){h()(this,"todos",[]),(0,o.makeAutoObservable)(this)}addTodo(e){this.todos.push(new u(e))}}let x=(0,l.observer)(({todo:e})=>(0,d.jsxs)("li",{children:[(0,d.jsx)("input",{type:"checkbox",checked:e.done,onChange:()=>{e.toggle()}}),e.title]})),j=(0,l.observer)(()=>{let[e]=(0,n.useState)(()=>new c);return(0,d.jsxs)(d.Fragment,{children:[(0,d.jsxs)("form",{onSubmit:t=>{t.preventDefault(),e.addTodo(t.target.todo.value)},children:["New Todo: ",(0,d.jsx)("input",{type:"text",name:"todo"})," ",(0,d.jsx)("button",{type:"submit",children:"Add"})]}),(0,d.jsx)("ul",{children:e.todos.map(e=>(0,d.jsx)(x,{todo:e},e.id))}),"Tasks left: ",e.unfinishedTodoCount]})});(0,r.createRoot)(document.querySelector("#root")).render((0,d.jsx)(n.StrictMode,{children:(0,d.jsx)(()=>(0,d.jsxs)(d.Fragment,{children:[(0,d.jsx)(i,{}),(0,d.jsx)(j,{})]}),{})}))}}]);
//# sourceMappingURL=main.f272a57c.chunk.js.map