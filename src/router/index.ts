import { createRouter, createWebHashHistory } from "vue-router";
import WelcomeView from "@/views/WelcomeView.vue";
import LibraryView from "@/views/LibraryView.vue";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "welcome", component: WelcomeView },
    { path: "/library", name: "library", component: LibraryView },
  ],
});

export default router;
