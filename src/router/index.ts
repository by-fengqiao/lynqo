import { createRouter, createWebHistory } from "vue-router";
import type { RouteRecordRaw } from "vue-router";

const routes: RouteRecordRaw[] = [
  {
    path: "/",
    component: () => import("../layouts/DesktopLayout.vue"),
    children: [
      {
        path: "",
        name: "home",
        component: () => import("../pages/HomePage.vue"),
      },
      {
        path: "transfers",
        name: "transfers",
        component: () => import("../pages/TransfersPage.vue"),
      },
      {
        path: "completed",
        redirect: { name: "transfers", query: { filter: "completed" } },
      },
      {
        path: "received",
        name: "received",
        component: () => import("../pages/ReceivedFilesPage.vue"),
      },
      {
        path: "devices",
        name: "devices",
        component: () => import("../pages/DevicesPage.vue"),
      },
      {
        path: "settings",
        name: "settings",
        component: () => import("../pages/SettingsPage.vue"),
      },
      {
        path: "about",
        name: "about",
        component: () => import("../pages/AboutPage.vue"),
      },
      {
        path: "help",
        name: "help",
        component: () => import("../pages/HelpPage.vue"),
      },
      {
        path: "legal/:documentType(privacy|terms|disclaimer)",
        name: "legal-document",
        component: () => import("../pages/LegalDocumentPage.vue"),
        props: true,
      },
      {
        path: ":pathMatch(.*)*",
        name: "not-found",
        component: () => import("../pages/NotFoundPage.vue"),
      },
    ],
  },
  {
    path: "/mobile",
    component: () => import("../layouts/MobileLayout.vue"),
    children: [
      {
        path: "",
        name: "mobile-send",
        component: () => import("../pages/MobileSendPage.vue"),
      },
      {
        path: "transfers",
        name: "mobile-transfers",
        component: () => import("../pages/MobileTransferPage.vue"),
      },
    ],
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
