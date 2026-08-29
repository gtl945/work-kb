<template>
  <div class="import-view">
    <h2>文件导入与提炼</h2>
    <p class="muted">
      选择本地办公文件（Word / Excel / PPT / PDF），自动解析 → 标题切块 → 规则抽字段，确认后入库。
    </p>

    <div class="picker-row">
      <el-button type="primary" :loading="loading" @click="onPick">
        <el-icon style="margin-right: 6px"><Upload /></el-icon>
        选择文件
      </el-button>
      <span v-if="fileList.length" class="file-count">
        已选 {{ fileList.length }} 个文件
      </span>
    </div>

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
          <el-tag size="small" type="danger">失败 - 查看详情</el-tag>
        </el-tooltip>
      </div>
    </div>

    <!-- 空状态：尚未选择任何文件 -->
    <el-empty
      v-if="!fileList.length && !err"
      description="点击上方按钮选择办公文件，支持批量导入"
    >
      <template #footer>
        <el-button type="primary" @click="onPick">选择文件</el-button>
      </template>
    </el-empty>

    <!-- 空状态：文件已解析但未提取到草稿 -->
    <el-alert
      v-if="fileList.length && !drafts.length && !loading && allParsed"
      type="info"
      title="未从文件中提取到草稿条目"
      description="可能原因：文件内容为空、无标题层级、或格式不标准。可尝试手动导入其他文件。"
      show-icon
      :closable="false"
      style="margin-top: 14px"
    />

    <!-- 草稿编辑区 -->
    <div v-if="drafts.length" class="drafts">
      <div class="toolbar">
        <span>共 {{ drafts.length }} 条草稿</span>
        <el-button type="primary" :loading="confirming" @click="onConfirm">
          确认入库
        </el-button>
      </div>
      <el-card v-for="(d, i) in drafts" :key="i" class="draft" shadow="never">
        <div class="draft-source">
          <el-tag size="small" type="info" effect="plain">{{ d._sourceName }}</el-tag>
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
import { Upload, Document } from "@element-plus/icons-vue";
import { pickFiles, importFiles, parseFile, confirmItems, type DraftItem } from "@/api";

interface FileEntry {
  name: string;
  path: string;
  status: "pending" | "parsing" | "done" | "error";
  draftCount: number;
  errorMsg?: string;
}

interface DraftRow extends DraftItem {
  _sourceName: string;
}

const loading = ref(false);
const confirming = ref(false);
const err = ref("");
const fileList = ref<FileEntry[]>([]);
const drafts = ref<DraftRow[]>([]);
const tagsText = ref<string[]>([]);

const allParsed = computed(() =>
  fileList.value.length > 0 && fileList.value.every((f) => f.status === "done" || f.status === "error")
);

async function onPick() {
  err.value = "";
  const paths = await pickFiles();
  if (!paths) return;

  const entries: FileEntry[] = paths.map((p) => {
    const name = p.split(/[\\/]/).pop() || p;
    return { name, path: p, status: "pending" as const, draftCount: 0 };
  });
  fileList.value = entries;
  drafts.value = [];
  tagsText.value = [];

  loading.value = true;
  try {
    const ids = await importFiles(paths);

    for (let fi = 0; fi < ids.length; fi++) {
      fileList.value[fi].status = "parsing";
      try {
        const list = await parseFile(ids[fi]);
        const rows: DraftRow[] = list.map((d) => ({
          ...d,
          _sourceName: entries[fi].name,
        }));
        drafts.value.push(...rows);
        tagsText.value.push(...rows.map((d) => (d.tags || []).join(", ")));
        fileList.value[fi].status = "done";
        fileList.value[fi].draftCount = list.length;
      } catch (e) {
        fileList.value[fi].status = "error";
        fileList.value[fi].errorMsg = String(e);
        console.error(`解析 ${entries[fi].name} 失败:`, e);
      }
    }

    if (drafts.value.length) {
      ElMessage.success(`提炼完成，共 ${drafts.value.length} 条草稿`);
    } else {
      ElMessage.warning("未提取到任何草稿条目，请检查文件内容或格式");
    }
  } catch (e) {
    err.value = `文件登记失败：${String(e)}。请确认文件路径可访问后重试。`;
  } finally {
    loading.value = false;
  }
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
.file-count {
  color: #6b7588;
  font-size: 14px;
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
.drafts {
  margin-top: 18px;
}
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
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
