import { createApp } from 'vue'
import { createRouter, createWebHistory } from 'vue-router'
import '@ui/styles.css'
import App from './App.vue'
import Home from './views/Home.vue'

const router = createRouter({
  history: createWebHistory('/accounts/'),
  routes: [
    { path: '/', component: Home },
    { path: '/profile', component: () => import('./views/Profile.vue') },
    { path: '/security', component: () => import('./views/Security.vue') },
    { path: '/sessions', component: () => import('./views/Sessions.vue') },
    { path: '/sessions/:id', component: () => import('./views/SessionDetail.vue'), meta: { parent: '/sessions' } },
    { path: '/activity', component: () => import('./views/Activity.vue') },
    { path: '/language', component: () => import('./views/Language.vue') },
    { path: '/users', component: () => import('./views/Users.vue'), meta: { admin: true } },
    { path: '/all-sessions', component: () => import('./views/AllSessions.vue'), meta: { admin: true } },
    { path: '/approve/:id', component: () => import('./views/Approve.vue'), meta: { bare: true } },
    { path: '/:rest(.*)*', redirect: '/' },
  ],
  scrollBehavior: () => ({ top: 0 }),
})

createApp(App).use(router).mount('#app')
