<template>
  <div class="kb-view">
    <h2>知识库</h2>

    <!-- 统计概览 -->
    <div class="stats">
      <el-card class="stat-card">
        <div class="stat-title">总条目</div>
        <div class="stat-value">{{ stats.total }}</div>
      </el-card>
      <el-card class="stat-card">
        <div class="stat-title">完成</div>
        <div class="stat-value">{{ stats.completed }}</div>
      </el-card>
      <el-card class="stat-card">
        <div class="stat-title">成果</div>
        <div class="stat-value">{{ stats.achievements }}</div>
      </el-card>
      <el-card class="stat-card">
        <div class="stat-title">亮点</div>
        <div class="stat-value">{{ stats.highlights }}</div>
      </el-card>
      <el-card class="stat-card">
        <div class="stat-title">项目</div>
        <div class="stat-value">{{ stats.projectCount }}</div>
      </el-card>
      <el-card class="stat-card">
        <div class="stat-title">源文件</div>
        <div class="stat-value">{{ stats.fileCount }}</div>
      </el-card>
    </div>

    <!-- 搜索栏 -->
    <el-input
      v-model="kw"
      placeholder="输入关键词（支持中文分词、拼音前缀、模糊匹配）"
      @keyup.enter="onSearch"
      clearable
    >
      <template #append>
        <el-button :loading="loading" @click="onSearch">搜索</el-button>
      </template>
    </el-input>

    <!-- 筛选条件 -->
    <div class="filters">
      <el-select
        v-model="filters.itemType"
        placeholder="类型"
        clearable
        style="width: 110px"
        @change="onSearch"
      >
        <el-option label="完成" value="完成" />
        <el-option label="成果" value="成果" />
        <el-option label="问题" value="问题" />
        <el-option label="亮点" value="亮点" />
      </el-select>
      <el-select
        v-model="filters.projectId"
        placeholder="项目"
        clearable
        style="width: 160px"
        @change="onSearch"
      >
        <el-option
          v-for="p in projects"
          :key="p.id"
          :label="`${p.name} (${p.itemCount})`"
          :value="p.id"
        />
      </el-select>
      <el-date-picker
        v-model="filters.dateFrom"
        type="date"
        value-format="YYYY-MM-DD"
        placeholder="开始日期"
        style="width: 145px"
        @change="onSearch"
      />
      <el-date-picker
        v-model="filters.dateTo"
        type="date"
        value-format="YYYY-MM-DD"
        placeholder="结束日期"
        style="width: 145px"
        @change="onSearch"
      />
      <el-select
        v-model="filters.evidenceType"
        placeholder="证据类型"
        clearable
        style="width: 140px"
        @change="onSearch"
      >
        <el-option label="专利" value="专利" />
        <el-option label="论文" value="论文" />
        <el-option label="奖项" value="奖项" />
        <el-option label="证书" value="证书" />
      </el-select>
    </div>

    <!-- 结果统计 -->
    <div v-if="searched" class="result-count">
      <span v-if="results.length">找到 {{ results.length }} 条结果</span>
      <span v-else class="muted">未找到匹配条目</span>
    </div>

    <!-- 结果列表 -->
    <div v-if="results.length" class="results">
      <el-card
        v-for="r in results"
        :key="r.id"
        class="result-card"
        shadow="hover"
      >
        <div class="result-header">
          <el-tag :type="typeTag(r.itemType)" size="small">{{ r.itemType }}</el-tag>
          <span class="result-title">{{ r.title }}</span>
          <span v-if="r.occurDate" class="result-date">{{ r.occurDate }}</span>
          <el-button
            :icon="Delete"
            size="small"
            type="text"
            class="delete-btn"
            @click="onDelete(r.id)"
          />
        </div>
        <div class="result-meta">
          <span v-if="r.projectName" class="meta-item">
            项目: {{ r.projectName }}
          </span>
          <el-tag v-if="r.quantValue" type="warning" size="small">
            量化: {{ r.quantValue }}
          </el-tag>
          <el-tag v-if="r.evidenceType" type="success" size="small">
            {{ r.evidenceType }}
          </el-tag>
          <el-tag
            v-for="t in r.tags"
            :key="t"
            size="small"
            effect="plain"
          >
            {{ t }}
          </el-tag>
        </div>
        <div class="result-points">{{ r.pointsText }}</div>
        <div v-if="r.sourceFilePath" class="result-footer">
          <el-button link type="primary" @click="onOpenSource(r.id)">
            <el-icon style="margin-right: 4px"><Link /></el-icon>
            {{ r.sourceFileName || "打开源文件" }}
          </el-button>
        </div>
      </el-card>
    </div>

    <!-- 空状态 -->
    <el-empty
      v-if="searched && !results.length"
      description="未找到匹配条目，试试调整关键词或筛选条件"
    />
    <el-empty
      v-if="!searched && !results.length && stats.total === 0"
      description="知识库还是空的，快去导入文件吧！"
    >
      <template #footer>
        <el-button type="primary" @click="$router.push('/import')">去导入文件</el-button>
      </template>
    </el-empty>
    <el-empty
      v-else-if="!searched && !results.length && stats.total > 0"
      description="输入关键词搜索，或直接点搜索浏览全部条目"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { Link, Delete } from "@element-plus/icons-vue";
import {
  searchItems,
  openSourceFile,
  deleteItem,
  getStats,
  getProjects,
  type SearchResult,
  type SearchFilters,
  type StatsResult,
  type ProjectInfo,
} from "@/api";

const kw = ref("");
const loading = ref(false);
const searched = ref(false);
const results = ref<SearchResult[]>([]);
const filters = reactive<SearchFilters>({
  itemType: null,
  dateFrom: null,
  dateTo: null,
  evidenceType: null,
  projectId: null,
});

const stats = ref<StatsResult>({
  total: 0,
  completed: 0,
  achievements: 0,
  problems: 0,
  highlights: 0,
  fileCount: 0,
  projectCount: 0,
  tagCount: 0,
});
const projects = ref<ProjectInfo[]>([]);

onMounted(async () => {
  try {
    const [s, p] = await Promise.all([getStats(), getProjects()]);
    stats.value = s;
    projects.value = p;
  } catch (e) {
    ElMessage.error("加载统计数据失败");
  }
});

function typeTag(t: string): string {
  switch (t) {
    case "完成":
      return "success";
    case "成果":
      return "primary";
    case "问题":
      return "warning";
    case "亮点":
      return "danger";
    default:
      return "info";
  }
}

async function onSearch() {
  loading.value = true;
  searched.value = true;
  try {
    results.value = await searchItems(kw.value.trim(), { ...filters });
  } catch (e) {
    ElMessage.error(String(e));
    results.value = [];
  } finally {
    loading.value = false;
  }
}

async function onOpenSource(itemId: number) {
  try {
    const ok = await openSourceFile(itemId);
    if (!ok) {
      ElMessage.warning("该条目未关联源文件");
    }
  } catch (e) {
    ElMessage.error(String(e));
  }
}

async function onDelete(itemId: number) {
  try {
    await ElMessageBox.confirm("确定要删除这条条目吗？", "确认删除", {
      confirmButtonText: "删除",
      cancelButtonText: "取消",
      type: "warning",
    });
  } catch {
    return;
  }

  try {
    await deleteItem(itemId);
    const item = results.value.find((r) => r.id === itemId);
    results.value = results.value.filter((r) => r.id !== itemId);
    stats.value.total--;
    if (item) {
      switch (item.itemType) {
        case "完成":
          stats.value.completed--;
          break;
        case "成果":
          stats.value.achievements--;
          break;
        case "问题":
          stats.value.problems--;
          break;
        case "亮点":
          stats.value.highlights--;
          break;
      }
    }
    ElMessage.success("删除成功");
  } catch (e) {
    ElMessage.error(String(e));
  }
}
</script>

<style scoped>
.kb-view h2 {
  margin-bottom: 18px;
}
.stats {
  display: flex;
  gap: 12px;
  margin-bottom: 20px;
  flex-wrap: wrap;
}
.stat-card {
  flex: 1;
  min-width: 90px;
  text-align: center;
}
.stat-title {
  font-size: 13px;
  color: #909399;
}
.stat-value {
  font-size: 24px;
  font-weight: 700;
  color: #303133;
  margin-top: 4px;
}
.filters {
  display: flex;
  gap: 10px;
  margin-top: 14px;
  flex-wrap: wrap;
}
.result-count {
  margin: 16px 0 8px;
  font-size: 14px;
}
.muted {
  color: #909399;
}
.results {
  margin-top: 8px;
}
.result-card {
  margin-bottom: 14px;
}
.result-header {
  display: flex;
  align-items: center;
  gap: 10px;
}
.result-title {
  font-weight: 600;
  font-size: 15px;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.result-date {
  color: #909399;
  font-size: 13px;
  white-space: nowrap;
}
.delete-btn {
  color: #f56c6c;
}
.result-meta {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
  margin-top: 10px;
}
.meta-item {
  font-size: 13px;
  color: #606266;
}
.result-points {
  margin-top: 10px;
  font-size: 14px;
  color: #303133;
  line-height: 1.6;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.result-footer {
  margin-top: 10px;
  padding-top: 8px;
  border-top: 1px solid #f0f2f6;
}
</style>
