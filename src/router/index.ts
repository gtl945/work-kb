import { createRouter, createWebHistory } from "vue-router";
import ImportView from "@/views/ImportView.vue";
import KnowledgeBaseView from "@/views/KnowledgeBaseView.vue";
import ExportView from "@/views/ExportView.vue";
import DatabaseView from "@/views/DatabaseView.vue";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", redirect: "/import" },
    { path: "/import", component: ImportView },
    { path: "/kb", component: KnowledgeBaseView },
    { path: "/export", component: ExportView },
    { path: "/database", component: DatabaseView },
  ],
});

export default router;
