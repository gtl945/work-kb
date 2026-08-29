<template>
  <div class="import-view">
    <h2>文件导入与提炼</h2>
    <p class="muted">
      选择文件或文件夹，自动解析 → 标题切块 → 规则抽字段，确认后入库。支持递归扫描子文件夹。
    </p>

    <div class="picker-row">
      <el-button type="primary" :loading="loading || scanning" @click="onPick">
        <el-icon style="margin-right: 6px"><Upload /></el-icon>
        选择文件
      </el-button>
      <el-button type="success" :loading="loading || scanning" @click="onPickFolder">
        <el-icon style="margin-right: 6px"><FolderOpened /></el-icon>
        选择文件夹
      </el-button>
    </div>

    <!-- 扫描摘要 -->
    <el-alert
      v-if="scanSummary"
      :title="scanSummary"
      type="info"
      show-icon
      :closable="true"
      style="margin-top: 14px"
    >
      <template #default>
        <div class="scan-actions" v-if="pendingFiles.length > 0">
          <el-button type="primary" size="small" :loading="loading" @click="onBatchParse">
            开始解析（{{ pendingFiles.length }} 个）
          </el-button>
        </div>
      </template>
    </el-alert>

    <!-- 解析进度 -->
    <el-progress
      v-if="loading && totalParse > 0"
      :percentage="Math.round((parsedCount / totalParse) * 100)"
      :status="parsedCount === totalParse ? 'success' : ''"
      style="margin-top: 14px"
    />

    <el-alert
      v-if="err"
      type="error"
      :title="err"
      show-icon
      :closable="false"
      style="margin-top: 14px"
    />

    <!-- 文件列表 + 解析状态 -->
    <div v-if="fileList.length" class="file-list">
      <div v-for="(f, i) in fileList" :key="i" class="file-item">
        <el-icon><Document /></el-icon>
        <span class="file-name" :title="f.path">{{ f.name }}</span>
        <el-tag v-if="f.status === 'pending'" size="small" type="info">待解析</el-tag>
        <el-tag v-if="f.status === 'parsing'" size="small" type="warning">解析中…</el-tag>
        <el-tag v-if="f.status === 'done'" size="small" type="success">
          {{ f.draftCount }} 条草稿
        </el-tag>
        <el-tooltip v-if="f.status === 'error'" :content="f.errorMsg || '解析失败'" placement="top">
          <el-tag size="small" type="danger">失败</el-tag>
        </el-tooltip>
        <el-tooltip v-if="f.status === 'skipped'" :content="f.errorMsg || '已跳过'" placement="top">
          <el-tag size="small" type="warning">跳过</el-tag>
        </el-tooltip>
      </div>
    </div>

    <!-- 空状态 -->
    <el-empty
      v-if="!fileList.length && !err && !scanSummary"
      description="点击上方按钮选择文件或文件夹，支持批量导入"
    >
      <template #footer>
        <div class="empty-actions">
          <el-button type="primary" @click="onPick">选择文件</el-button>
          <el-button type="success" @click="onPickFolder">选择文件夹</el-button>
        </div>
      </template>
    </el-empty>

    <!-- 草稿编辑区 -->
    <div v-if="drafts.length" class="drafts">
      <div class="toolbar">
        <span>
          共 {{ drafts.length }} 条草稿
          <el-tag v-if="fallbackCount > 0" size="small" type="danger" style="margin-left: 8px">
            {{ fallbackCount }} 条解析失败·自动生成
          </el-tag>
          <el-tag v-if="dupCount > 0" size="small" type="warning" style="margin-left: 8px">
            {{ dupCount }} 条疑似重复
          </el-tag>
        </span>
        <div class="toolbar-actions">
          <el-button v-if="dupCount > 0" size="small" @click="onSkipDuplicates">
            跳过重复（{{ dupCount }}）
          </el-button>
          <el-button type="primary" :loading="confirming" @click="onConfirm">
            全部确认入库
          </el-button>
        </div>
      </div>
      <el-card v-for="(d, i) in drafts" :key="i" class="draft" shadow="never">
        <div class="draft-source">
          <el-tag size="small" type="info" effect="plain">{{ d._sourceName }}</el-tag>
          <el-tag v-if="d._isFallback" size="small" type="danger" style="margin-left: 8px">
            解析失败·自动生成
          </el-tag>
          <el-tag v-if="d._isDuplicate" size="small" type="warning" style="margin-left: 8px">
            疑似重复
          </el-tag>
        </div>
        <div class="row">
          <el-select v-model="d.itemType" style="width: 110px">
            <el-option label="完成" value="完成" />
            <el-option label="成果" value="成果" />
            <el-option label="问题" value="问题" />
            <el-option label="亮点" value="亮点" />
          </el-select>
          <el-input v-model="d.title" placeholder="标题" style="flex: 1" />
          <el-date-picker
            v-model="d.occurDate"
            type="date"
            value-format="YYYY-MM-DD"
            placeholder="日期"
            style="width: 150px"
          />
        </div>
        <div class="row" style="margin-top: 10px">
          <el-input v-model="d.project" placeholder="项目（选填）" style="width: 200px" />
          <el-input v-model="tagsText[i]" placeholder="标签，逗号分隔" style="flex: 1" />
          <el-input
            v-if="d.itemType === '亮点'"
            v-model="d.evidenceType"
            placeholder="证据类型(专利/论文/奖项)"
            style="width: 220px"
          />
          <el-tag v-if="d.quantValue" type="warning" size="small">
            量化: {{ d.quantValue }}
          </el-tag>
        </div>
        <el-input
          v-model="d.pointsText"
          type="textarea"
          :autosize="{ minRows: 2, maxRows: 6 }"
          class="points"
          placeholder="要点正文"
        />
      </el-card>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import { ElMessage } from "element-plus";
import { Upload, Document, FolderOpened } from "@element-plus/icons-vue";
import {
  pickFiles,
  pickFolder,
  importFiles,
  parseFile,
  confirmItems,
  scanFolder,
  checkItemDuplicate,
  type DraftItem,
  type ScannedFile,
} from "@/api";

interface FileEntry {
  name: string;
  path: string;
  status: "pending" | "parsing" | "done" | "error" | "skipped";
  draftCount: number;
  errorMsg?: string;
}

interface DraftRow extends DraftItem {
  _sourceName: string;
  _isDuplicate: boolean;
  _isFallback: boolean;
}

const loading = ref(false);
const scanning = ref(false);
const confirming = ref(false);
const err = ref("");
const fileList = ref<FileEntry[]>([]);
const drafts = ref<DraftRow[]>([]);
const tagsText = ref<string[]>([]);
const scanSummary = ref("");
const parsedCount = ref(0);
const totalParse = ref(0);

const pendingFiles = computed(() =>
  fileList.value.filter((f) => f.status === "pending")
);

const dupCount = computed(() =>
  drafts.value.filter((d) => d._isDuplicate).length
);

const fallbackCount = computed(() =>
  drafts.value.filter((d) => d._isFallback).length
);

async function onPick() {
  err.value = "";
  scanSummary.value = "";
  const paths = await pickFiles();
  if (!paths) return;

  fileList.value = paths.map((p) => {
    const name = p.split(/[\\/]/).pop() || p;
    return { name, path: p, status: "pending" as const, draftCount: 0 };
  });
  drafts.value = [];
  tagsText.value = [];

  await doImportAndParse();
}

async function onPickFolder() {
  err.value = "";
  scanSummary.value = "";
  const folderPath = await pickFolder();
  if (!folderPath) return;

  scanning.value = true;
  try {
    const scanned: ScannedFile[] = await scanFolder(folderPath);
    if (!scanned.length) {
      ElMessage.warning("未在文件夹中找到支持的文件格式");
      return;
    }

    const registered = scanned.filter((s) => s.alreadyRegistered);
    const pending = scanned.filter((s) => !s.alreadyRegistered);

    fileList.value = scanned.map((s) => ({
      name: s.filename,
      path: s.path,
      status: s.alreadyRegistered ? ("skipped" as const) : ("pending" as const),
      draftCount: 0,
      errorMsg: s.alreadyRegistered ? "已登记" : undefined,
    }));

    scanSummary.value = `找到 ${scanned.length} 个文件，已登记 ${registered.length} 个，待解析 ${pending.length} 个`;
    drafts.value = [];
    tagsText.value = [];
  } catch (e) {
    err.value = `文件夹扫描失败：${String(e)}`;
  } finally {
    scanning.value = false;
  }
}

async function onBatchParse() {
  await doImportAndParse();
}

async function doImportAndParse() {
  const toProcess = fileList.value.filter((f) => f.status === "pending");
  if (!toProcess.length) return;

  loading.value = true;
  err.value = "";
  totalParse.value = toProcess.length;
  parsedCount.value = 0;

  try {
    const paths = toProcess.map((f) => f.path);
    const results = await importFiles(paths);

    for (let fi = 0; fi < results.length; fi++) {
      const result = results[fi];
      const fileEntry = toProcess[fi];
      const fileIdx = fileList.value.indexOf(fileEntry);

      if (!result.isNew) {
        fileList.value[fileIdx].status = "skipped";
        fileList.value[fileIdx].errorMsg = result.duplicateReason || "重复";
        parsedCount.value++;
        continue;
      }

      fileList.value[fileIdx].status = "parsing";
      try {
        const list = await parseFile(result.fileId);
        const sourceName = fileEntry.name;

        for (const d of list) {
          const isDup = await checkItemDuplicate(
            d.title,
            d.occurDate
          ).catch(() => false);

          drafts.value.push({ ...d, _sourceName: sourceName, _isDuplicate: isDup, _isFallback: d.isFallback || false });
          tagsText.value.push((d.tags || []).join(", "));
        }

        fileList.value[fileIdx].status = "done";
        fileList.value[fileIdx].draftCount = list.length;
      } catch (e) {
        fileList.value[fileIdx].status = "error";
        fileList.value[fileIdx].errorMsg = String(e);
      }
      parsedCount.value++;
    }

    if (drafts.value.length) {
      const fbMsg = fallbackCount.value ? `，${fallbackCount.value} 条解析失败已自动生成` : "";
      const dupMsg = dupCount.value ? `，${dupCount.value} 条疑似重复` : "";
      ElMessage.success(`提炼完成，共 ${drafts.value.length} 条草稿${fbMsg}${dupMsg}`);
    } else {
      ElMessage.warning("未提取到任何草稿条目，请检查文件内容或格式");
    }
  } catch (e) {
    err.value = `文件登记失败：${String(e)}`;
  } finally {
    loading.value = false;
  }
}

function onSkipDuplicates() {
  const keep = drafts.value.filter((d) => !d._isDuplicate);
  const removed = drafts.value.length - keep.length;
  drafts.value = keep;
  tagsText.value = tagsText.value.slice(0, keep.length);
  ElMessage.success(`已跳过 ${removed} 条重复草稿`);
}

async function onConfirm() {
  confirming.value = true;
  try {
    const payload: DraftItem[] = drafts.value.map((d, i) => ({
      title: d.title,
      itemType: d.itemType,
      occurDate: d.occurDate,
      project: d.project,
      pointsText: d.pointsText,
      quantValue: d.quantValue,
      sourceFileId: d.sourceFileId,
      evidenceType: d.evidenceType,
      tags: (tagsText.value[i] || "")
        .split(/[,，]/)
        .map((s) => s.trim())
        .filter(Boolean),
    }));
    const ids = await confirmItems(payload);
    ElMessage.success(`已入库 ${ids.length} 条`);
    drafts.value = [];
    tagsText.value = [];
    fileList.value = [];
    scanSummary.value = "";
  } catch (e) {
    ElMessage.error(`入库失败：${String(e)}`);
  } finally {
    confirming.value = false;
  }
}
</script>

<style scoped>
.muted {
  color: #6b7588;
  margin: 8px 0 18px;
}
.picker-row {
  display: flex;
  align-items: center;
  gap: 14px;
}
.scan-actions {
  margin-top: 8px;
}
.file-list {
  margin-top: 16px;
}
.file-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 0;
  border-bottom: 1px solid #f0f2f6;
}
.file-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 14px;
}
.empty-actions {
  display: flex;
  gap: 12px;
  justify-content: center;
}
.drafts {
  margin-top: 18px;
}
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}
.toolbar-actions {
  display: flex;
  gap: 8px;
}
.draft {
  margin-bottom: 12px;
}
.draft-source {
  margin-bottom: 8px;
}
.row {
  display: flex;
  gap: 10px;
  align-items: center;
}
.points {
  margin-top: 10px;
}
</style>
