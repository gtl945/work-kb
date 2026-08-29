<template>
  <div class="export-view">
    <h2>报告导出</h2>
    <p class="muted">
      选择粒度和时间范围，从知识库条目生成日报 / 周报 / 季报 / 年报 Markdown。
    </p>

    <div class="controls">
      <el-radio-group v-model="granularity" @change="onGenerate">
        <el-radio-button value="daily">日报</el-radio-button>
        <el-radio-button value="weekly">周报</el-radio-button>
        <el-radio-button value="quarterly">季报</el-radio-button>
        <el-radio-button value="yearly">年报</el-radio-button>
      </el-radio-group>

      <el-date-picker
        v-model="dateFrom"
        type="date"
        value-format="YYYY-MM-DD"
        placeholder="开始日期"
        style="width: 145px"
        @change="onGenerate"
      />
      <span class="tilde">~</span>
      <el-date-picker
        v-model="dateTo"
        type="date"
        value-format="YYYY-MM-DD"
        placeholder="结束日期"
        style="width: 145px"
        @change="onGenerate"
      />

      <el-button type="primary" :loading="loading" @click="onGenerate">
        生成报告
      </el-button>
    </div>

    <el-alert
      v-if="err"
      type="error"
      :title="err"
      show-icon
      :closable="false"
      style="margin-top: 14px"
    />

    <div v-if="result && result.itemCount > 0" class="result-section">
      <div class="result-toolbar">
        <span class="result-info">
          {{ result.itemCount }} 个条目 | {{ result.fileList.length }} 个源文件 | {{ markdownLines }} 行
        </span>
        <div class="btns">
          <el-button size="small" @click="onCopy">复制</el-button>
          <el-button size="small" type="primary" @click="onSave">保存为 .md</el-button>
        </div>
      </div>

      <div v-if="result.fileList.length" class="file-list">
        <span class="muted">源文件: </span>
        <el-tag
          v-for="f in result.fileList"
          :key="f"
          size="small"
          effect="plain"
          style="margin-right: 6px"
        >
          {{ f }}
        </el-tag>
      </div>

      <el-input
        v-model="result.markdown"
        type="textarea"
        :autosize="{ minRows: 10, maxRows: 30 }"
        class="preview"
        readonly
      />
    </div>

    <el-empty
      v-if="result && result.itemCount === 0"
      description="该时间范围内没有条目，试试调整日期范围或粒度"
    />
    <el-empty
      v-if="!result && !loading"
      description="选择粒度和日期范围后点击「生成报告」"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import { ElMessage } from "element-plus";
import {
  exportView,
  saveFile,
  pickSavePath,
  type ExportGranularity,
  type ExportResult,
} from "@/api";

const granularity = ref<ExportGranularity>("weekly");
const dateFrom = ref<string | null>(null);
const dateTo = ref<string | null>(null);
const loading = ref(false);
const err = ref("");
const result = ref<ExportResult | null>(null);

const markdownLines = computed(() => {
  if (!result.value) return 0;
  return result.value.markdown.split("\n").length;
});

async function onGenerate() {
  loading.value = true;
  err.value = "";
  result.value = null;
  try {
    result.value = await exportView({
      granularity: granularity.value,
      dateFrom: dateFrom.value,
      dateTo: dateTo.value,
    });
    if (result.value.itemCount === 0) {
      ElMessage.warning("该时间范围内没有条目");
    }
  } catch (e) {
    err.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function onCopy() {
  if (!result.value) return;
  try {
    await navigator.clipboard.writeText(result.value.markdown);
    ElMessage.success("已复制到剪贴板");
  } catch {
    ElMessage.warning("剪贴板不可用，请手动选择文本复制");
  }
}

async function onSave() {
  if (!result.value) return;
  const defaultName = `工作${labelOf(granularity.value)}.md`;
  try {
    const path = await pickSavePath(defaultName);
    if (!path) return;
    await saveFile(path, result.value.markdown);
    ElMessage.success("已保存");
  } catch (e) {
    ElMessage.error(String(e));
  }
}

function labelOf(g: ExportGranularity): string {
  return { daily: "日报", weekly: "周报", quarterly: "季报", yearly: "年报" }[g];
}
</script>

<style scoped>
.muted {
  color: #6b7588;
}
.export-view .muted {
  margin: 8px 0 18px;
}
.controls {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.tilde {
  color: #909399;
}
.result-section {
  margin-top: 18px;
}
.result-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}
.result-info {
  font-size: 13px;
  color: #606266;
}
.btns {
  display: flex;
  gap: 8px;
}
.file-list {
  margin-bottom: 12px;
}
.preview {
  margin-top: 4px;
}
</style>
