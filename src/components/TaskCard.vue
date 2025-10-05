<template>
  <el-card class="task-card" shadow="never">
    <div class="task-header">
      <div class="task-title-group">
        <span class="task-title">{{ task.name }}</span>
        <el-tag size="small" type="info">{{ typeLabel }}</el-tag>
        <el-tag v-if="statusLabel" size="small" :type="statusTagType">{{ statusLabel }}</el-tag>
      </div>
      <div class="task-actions">
        <el-tooltip content="编辑任务" placement="top">
          <el-button :icon="Edit" circle size="small" @click="$emit('edit', task)" />
        </el-tooltip>
        <el-tooltip content="删除任务" placement="top">
          <el-button :icon="Delete" circle size="small" @click="$emit('delete', task.id)" />
        </el-tooltip>
      </div>
    </div>

    <p v-if="task.description" class="task-desc">{{ task.description }}</p>

    <div class="task-meta">
      <span class="meta-item" v-if="task.repeatRule">周期：{{ repeatLabel }}</span>
      <span class="meta-item" v-else-if="task.startDate || task.endDate">
        {{ task.startDate || "未设定" }} → {{ task.endDate || "未设定" }}
      </span>
      <span class="meta-item">目标：{{ task.progress }} / {{ task.target }}</span>
      <span class="meta-item">最近更新：{{ latestUpdate }}</span>
    </div>

    <el-progress
      :percentage="progressPercentage"
      :status="progressStatus"
      :text-inside="true"
      stroke-width="18"
    />

    <div class="card-footer">
      <el-button
        size="small"
        type="primary"
        :disabled="isArchived || (isCompleted && task.type !== 'cycle')"
        @click="$emit('increase', task.id)"
      >
        +1 进度
      </el-button>

      <el-button
        v-if="shouldShowRestart"
        size="small"
        type="success"
        plain
        @click="$emit('restart', task.id)"
      >
        刷新周期
      </el-button>

      <el-button
        v-if="!isArchived"
        size="small"
        type="warning"
        text
        @click="$emit('archive', task.id)"
      >
        移动到归档
      </el-button>
      <el-button
        v-else
        size="small"
        type="info"
        text
        @click="$emit('reopen', task.id)"
      >
        重新激活
      </el-button>
    </div>
  </el-card>
</template>

<script setup>
import { computed } from "vue"
import { Edit, Delete } from "@element-plus/icons-vue"

const typeDisplay = {
  once: "一次性任务",
  cycle: "周期性任务",
  long_term: "长期任务"
}

const repeatDisplay = {
  daily: "每天",
  weekly: "每周",
  monthly: "每月"
}

const statusDisplay = {
  active: { label: "进行中", type: "" },
  completed: { label: "已完成", type: "success" },
  archived: { label: "已归档", type: "info" }
}

const props = defineProps({
  task: {
    type: Object,
    required: true
  }
})

defineEmits(["edit", "delete", "increase", "archive", "reopen", "restart"])

const typeLabel = computed(() => typeDisplay[props.task.type] ?? "未知类型")
const repeatLabel = computed(() => repeatDisplay[props.task.repeatRule] ?? props.task.repeatRule ?? "-")

const progressPercentage = computed(() => {
  const target = props.task.target || 1
  return Math.min(100, Math.round((props.task.progress / target) * 100))
})

const isCompleted = computed(() => props.task.status === "completed")
const isArchived = computed(() => props.task.status === "archived")
const shouldShowRestart = computed(() => props.task.type === "cycle" && isCompleted.value)
const progressStatus = computed(() => (isCompleted.value ? "success" : ""))

const statusLabel = computed(() => statusDisplay[props.task.status]?.label ?? "")
const statusTagType = computed(() => statusDisplay[props.task.status]?.type ?? "info")

const latestUpdate = computed(() => {
  if (!props.task.updatedAt) return "-"
  return props.task.updatedAt.slice(0, 10)
})
</script>

<style scoped>
.task-card {
  margin-bottom: 16px;
  padding: 16px;
  border-radius: 14px;
  border: 1px solid #eef1f5;
  background: #ffffff;
}

.task-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 12px;
}

.task-title-group {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.task-title {
  font-size: 18px;
  font-weight: 600;
  color: #1f2430;
}

.task-actions {
  display: flex;
  gap: 6px;
}

.task-desc {
  font-size: 14px;
  color: #5f6777;
  margin-bottom: 12px;
  line-height: 1.6;
}

.task-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  font-size: 12px;
  color: #7a8294;
  margin-bottom: 12px;
}

.meta-item {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.card-footer {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 12px;
}
</style>

