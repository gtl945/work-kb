<template>
  <el-container class="layout">
    <el-aside width="200px" class="aside">
      <div class="logo">工作知识库</div>
      <el-menu :default-active="activeMenu" router>
        <el-menu-item index="/import">
          <el-icon><Upload /></el-icon>
          <span>文件导入</span>
        </el-menu-item>
        <el-menu-item index="/kb">
          <el-icon><Search /></el-icon>
          <span>知识库</span>
        </el-menu-item>
        <el-menu-item index="/export">
          <el-icon><Document /></el-icon>
          <span>报告导出</span>
        </el-menu-item>
      </el-menu>
    </el-aside>
    <el-container>
      <el-header class="header">
        <span>个人工作知识库 v0.1</span>
        <el-tag size="small" :type="dbInfo ? 'success' : 'info'">
          DB: {{ dbInfo || "未连接" }}
        </el-tag>
      </el-header>
      <el-main>
        <router-view />
      </el-main>
    </el-container>
  </el-container>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import { useRoute } from "vue-router";
import { Upload, Search, Document } from "@element-plus/icons-vue";
import { dbStatus } from "@/api";

const route = useRoute();
const activeMenu = computed(() => route.path);
const dbInfo = ref("");

dbStatus()
  .then((s) => (dbInfo.value = s))
  .catch(() => (dbInfo.value = ""));
</script>

<style scoped>
.layout {
  height: 100vh;
}
.aside {
  background: #1b2335;
}
.logo {
  color: #fff;
  font-weight: 700;
  font-size: 18px;
  padding: 20px 18px;
}
.aside :deep(.el-menu) {
  background: transparent;
  border-right: none;
}
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: #fff;
  border-bottom: 1px solid #e4e9f2;
}
</style>
