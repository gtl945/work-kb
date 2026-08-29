<template>
  <div class="db-view">
    <h2>数据库状态</h2>

    <!-- 数据库信息 -->
    <el-card class="info-card" shadow="never">
      <template #header>数据库文件</template>
      <el-descriptions :column="1" border>
        <el-descriptions-item label="文件路径">
          <span class="mono">{{ dbInfo?.dbPath || "加载中..." }}</span>
        </el-descriptions-item>
        <el-descriptions-item label="文件大小">
          {{ dbInfo ? formatSize(dbInfo.dbSize) : "..." }}
        </el-descriptions-item>
        <el-descriptions-item label="数据持久化">
          <el-tag type="success" size="small">本地存储</el-tag>
          <span class="hint">重新安装或更新应用不会丢失数据</span>
        </el-descriptions-item>
      </el-descriptions>
    </el-card>

    <!-- 表统计 -->
    <el-card class="info-card" shadow="never">
      <template #header>表行数统计</template>
      <el-table v-if="dbInfo" :data="dbInfo.tables" stripe size="small">
        <el-table-column prop="name" label="表名" />
        <el-table-column prop="rowCount" label="行数" width="120" align="right" />
      </el-table>
      <el-skeleton v-else :rows="4" animated />
    </el-card>

    <!-- 数据备份与恢复 -->
    <el-card class="info-card" shadow="never">
      <template #header>数据备份与恢复</template>
      <div class="backup-actions">
        <el-button type="primary" :loading="exporting" @click="onExport">
          导出备份
        </el-button>
        <el-button type="warning" :loading="importing" @click="onImport">
          恢复数据
        </el-button>
      </div>
      <el-alert
        v-if="importDone"
        title="数据恢复成功，请刷新页面以加载新数据"
        type="success"
        :closable="true"
        show-icon
        style="margin-top: 12px"
      />
    </el-card>

    <!-- 已登记源文件 -->
    <el-card class="info-card" shadow="never">
      <template #header>
        <span>已登记源文件</span>
        <el-tag size="small" style="margin-left: 8px">
          {{ files.length }} 个
        </el-tag>
      </template>
      <el-table v-if="files.length" :data="files" stripe size="small">
        <el-table-column prop="filename" label="文件名" min-width="200" show-overflow-tooltip />
        <el-table-column prop="ext" label="格式" width="80" align="center" />
        <el-table-column label="大小" width="100" align="right">
          <template #default="{ row }">
            {{ row.size ? formatSize(row.size) : "-" }}
          </template>
        </el-table-column>
        <el-table-column prop="itemCount" label="关联条目" width="100" align="center" />
        <el-table-column label="导入时间" width="160">
          <template #default="{ row }">
            {{ formatTime(row.importedAt) }}
          </template>
        </el-table-column>
        <el-table-column prop="path" label="路径" min-width="300" show-overflow-tooltip />
      </el-table>
      <el-empty v-else-if="!loading" description="暂无已登记文件" />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import {
  getDbInfo,
  getFileList,
  exportData,
  importData,
  pickDbSavePath,
  pickDbOpenPath,
  type DbInfo,
  type SourceFileInfo,
} from "@/api";

const dbInfo = ref<DbInfo | null>(null);
const files = ref<SourceFileInfo[]>([]);
const loading = ref(true);
const exporting = ref(false);
const importing = ref(false);
const importDone = ref(false);

onMounted(async () => {
  await loadAll();
});

async function loadAll() {
  loading.value = true;
  try {
    const [info, fileList] = await Promise.all([getDbInfo(), getFileList()]);
    dbInfo.value = info;
    files.value = fileList;
  } catch (e) {
    ElMessage.error("加载数据库信息失败: " + String(e));
  } finally {
    loading.value = false;
  }
}

async function onExport() {
  const path = await pickDbSavePath();
  if (!path) return;

  exporting.value = true;
  try {
    await exportData(path);
    ElMessage.success("备份已导出到: " + path);
  } catch (e) {
    ElMessage.error("导出失败: " + String(e));
  } finally {
    exporting.value = false;
  }
}

async function onImport() {
  const path = await pickDbOpenPath();
  if (!path) return;

  try {
    await ElMessageBox.confirm(
      "恢复操作将覆盖当前所有数据，且不可撤销。确定继续吗？",
      "确认恢复",
      {
        confirmButtonText: "恢复",
        cancelButtonText: "取消",
        type: "warning",
      }
    );
  } catch {
    return;
  }

  importing.value = true;
  try {
    await importData(path);
    importDone.value = true;
    ElMessage.success("数据恢复成功");
    await loadAll();
  } catch (e) {
    ElMessage.error("恢复失败: " + String(e));
  } finally {
    importing.value = false;
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
  return (bytes / (1024 * 1024)).toFixed(2) + " MB";
}

function formatTime(unix: number): string {
  if (!unix) return "-";
  const d = new Date(unix * 1000);
  return d.toLocaleString("zh-CN");
}
</script>

<style scoped>
.db-view {
  max-width: 900px;
}
.info-card {
  margin-bottom: 16px;
}
.mono {
  font-family: monospace;
  font-size: 13px;
  word-break: break-all;
}
.hint {
  margin-left: 8px;
  color: #909399;
  font-size: 12px;
}
.backup-actions {
  display: flex;
  gap: 12px;
}
</style>
