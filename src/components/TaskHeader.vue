<template>
  <div class="task-header-wrapper">
    <div class="top-row">
      <div class="title-group">
        <h2 class="panel-title">TimeMaster</h2>
        <p class="panel-subtitle">一次性 · 周期性 · 长期任务统一管理</p>
      </div>
      <div class="time-block">
        <div class="date">{{ currentDate }}</div>
        <div class="time">{{ currentTime }}</div>
      </div>
    </div>

    <div class="main-buttons">
      <el-button
        :type="activeTab === 'tasks' ? 'primary' : 'info'"
        plain
        @click="$emit('changeTab', 'tasks')"
      >
        任务列表
      </el-button>
      <el-button
        :type="activeTab === 'archive' ? 'primary' : 'info'"
        plain
        @click="$emit('changeTab', 'archive')"
      >
        归档记录
      </el-button>
    </div>
  </div>
</template>

<script setup>
defineProps({
  currentDate: {
    type: String,
    default: ""
  },
  currentTime: {
    type: String,
    default: ""
  },
  activeTab: {
    type: String,
    default: "tasks"
  }
})

defineEmits(["changeTab"])
</script>

<style scoped>
.task-header-wrapper {
  display: flex;
  flex-direction: column;
  gap: 18px;
  margin-bottom: 8px;
}

.top-row {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.title-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.panel-title {
  font-size: 26px;
  font-weight: 700;
  color: #1f2430;
  margin: 0;
}

.panel-subtitle {
  margin: 0;
  font-size: 13px;
  color: #7c8295;
}

.time-block {
  text-align: right;
  color: #4e5566;
}

.time-block .date {
  font-size: 18px;
  margin-bottom: 2px;
}

.time-block .time {
  font-size: 32px;
  font-weight: 600;
}

.main-buttons {
  display: flex;
  gap: 12px;
}
</style>

