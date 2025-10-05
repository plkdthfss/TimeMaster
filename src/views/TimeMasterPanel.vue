<template>
  <div class="time-master-container">
    <div class="panel-content">
      <TaskHeader
        :current-date="currentDate"
        :current-time="currentTime"
        :active-tab="activeTab"
        @changeTab="handleTabChange"
      />

      <section class="overview">
        <div class="stat-card">
          <span class="stat-label">进行中</span>
          <span class="stat-value">{{ stats.active }}</span>
        </div>
        <div class="stat-card">
          <span class="stat-label">已完成</span>
          <span class="stat-value">{{ stats.completed }}</span>
        </div>
        <div class="stat-card">
          <span class="stat-label">归档</span>
          <span class="stat-value">{{ stats.archived }}</span>
        </div>
        <div class="stat-card">
          <span class="stat-label">完成率</span>
          <span class="stat-value">{{ stats.completionRate }}%</span>
        </div>
      </section>

      <section class="filter-bar">
        <span class="filter-label">类型筛选</span>
        <el-radio-group v-model="typeFilter" size="small">
          <el-radio-button label="all">全部</el-radio-button>
          <el-radio-button label="once">一次性</el-radio-button>
          <el-radio-button label="cycle">周期性</el-radio-button>
          <el-radio-button label="long_term">长期</el-radio-button>
        </el-radio-group>
      </section>

      <el-scrollbar class="task-scroll">
        <template v-if="activeTab === 'tasks'">
          <section v-if="filteredActiveTasks.length" class="task-section">
            <h3 class="section-title">进行中</h3>
            <TaskCard
              v-for="task in filteredActiveTasks"
              :key="task.id"
              :task="task"
              @increase="handleIncrease"
              @delete="handleDelete"
              @archive="handleArchive"
              @edit="handleEdit"
            />
          </section>
          <el-empty v-else description="暂无进行中的任务" />

          <section v-if="filteredCompletedTasks.length" class="task-section">
            <h3 class="section-title">已完成</h3>
            <TaskCard
              v-for="task in filteredCompletedTasks"
              :key="task.id"
              :task="task"
              @increase="handleIncrease"
              @delete="handleDelete"
              @archive="handleArchive"
              @reopen="handleReopen"
              @restart="handleRestart"
              @edit="handleEdit"
            />
          </section>
        </template>

        <template v-else>
          <section v-if="filteredArchivedTasks.length" class="task-section">
            <h3 class="section-title">归档记录</h3>
            <TaskCard
              v-for="task in filteredArchivedTasks"
              :key="task.id"
              :task="task"
              @delete="handleDelete"
              @reopen="handleReopen"
              @edit="handleEdit"
            />
          </section>
          <el-empty v-else description="归档列表为空" />
        </template>
      </el-scrollbar>
    </div>

    <el-button type="primary" circle class="fab-btn" @click="openTaskDialog">
      <el-icon><CirclePlus /></el-icon>
    </el-button>

    <AddTaskDialog
      v-model="taskDialogVisible"
      :mode="dialogMode"
      :task="editingTask"
      @save="handleTaskSave"
    />
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from "vue"
import { ElMessage, ElMessageBox } from "element-plus"
import { CirclePlus } from "@element-plus/icons-vue"
import { getCurrentWindow, LogicalPosition, LogicalSize, currentMonitor } from "@tauri-apps/api/window"
import TaskCard from "@/components/TaskCard.vue"
import AddTaskDialog from "@/components/AddTaskDialog.vue"
import TaskHeader from "@/components/TaskHeader.vue"
import { useClock } from "@/composables/useClock"
import { useTasks } from "@/composables/useTasks"

const taskDialogVisible = ref(false)
const dialogMode = ref("create")
const editingTask = ref(null)
const activeTab = ref("tasks")
const typeFilter = ref("all")

const { currentDate, currentTime } = useClock()
const {
  tasks,
  activeTasks,
  completedTasks,
  archivedTasks,
  stats,
  addTask,
  updateTask,
  deleteTask,
  increaseProgress,
  archiveTask,
  reopenTask,
  reloadTasks
} = useTasks()

const configureWindow = async () => {
  try {
    const monitor = await currentMonitor()
    if (!monitor) return

    const scale = monitor.scaleFactor ?? 1
    const size = monitor.size
    const logicalWidth = size.width / scale
    const logicalHeight = size.height / scale
    const targetWidth = Math.min(Math.max(logicalWidth * 0.32, 360), 520)

    const win = getCurrentWindow()
    await win.setResizable(false)
    await win.setDecorations(false)
    await win.setAlwaysOnTop(false)
    await win.setSize(new LogicalSize(targetWidth, logicalHeight))

    const position = monitor.position ?? { x: 0, y: 0 }
    const offsetX = position.x / scale
    const offsetY = position.y / scale
    await win.setPosition(new LogicalPosition(offsetX + logicalWidth - targetWidth, offsetY))
  } catch (error) {
    console.warn("configureWindow failed", error)
  }
}

onMounted(async () => {
  await configureWindow()
  await reloadTasks()
})

const filterByType = (list) => {
  if (typeFilter.value === "all") return list
  return list.filter(task => task.type === typeFilter.value)
}

const filteredActiveTasks = computed(() => filterByType(activeTasks.value))
const filteredCompletedTasks = computed(() => filterByType(completedTasks.value))
const filteredArchivedTasks = computed(() => filterByType(archivedTasks.value))

const findTaskById = (id) => tasks.value.find(task => task.id === id)

const handleTabChange = (tab) => {
  activeTab.value = tab
}

const openTaskDialog = () => {
  dialogMode.value = "create"
  editingTask.value = null
  taskDialogVisible.value = true
}

const handleTaskSave = async (payload) => {
  try {
    if (dialogMode.value === "edit" && editingTask.value) {
      await updateTask({ ...payload, id: payload.id ?? editingTask.value.id })
      ElMessage.success("任务已更新")
    } else {
      await addTask(payload)
      ElMessage.success("任务已创建")
    }
  } catch (error) {
    ElMessage.error("任务保存失败")
    console.error("failed to save task", error)
  }
}

const handleIncrease = async (id) => {
  const previousStatus = findTaskById(id)?.status
  await increaseProgress(id)
  const currentStatus = findTaskById(id)?.status
  if (previousStatus !== "completed" && currentStatus === "completed") {
    ElMessage.success("任务达成目标 🎉")
  }
}

const handleArchive = async (id) => {
  await archiveTask(id)
  ElMessage.info("任务已移动到归档")
}

const handleReopen = async (id) => {
  await reopenTask(id)
  ElMessage.success("任务已重新激活")
}

const handleRestart = async (id) => {
  await reopenTask(id)
  ElMessage.success("已刷新任务周期")
}

const handleDelete = async (id) => {
  try {
    await ElMessageBox.confirm("确认删除该任务？", "删除确认", {
      confirmButtonText: "删除",
      cancelButtonText: "取消",
      type: "warning"
    })
    await deleteTask(id)
    ElMessage.success("任务已删除")
  } catch (error) {
    if (error !== "cancel") {
      console.error("failed to delete task", error)
      ElMessage.error("删除失败")
    }
  }
}

const handleEdit = (task) => {
  dialogMode.value = "edit"
  editingTask.value = JSON.parse(JSON.stringify(task))
  taskDialogVisible.value = true
}
</script>

<style scoped>
.time-master-container {
  position: relative;
  width: 100%;
  height: 100vh;
  padding: 24px 28px 36px;
  box-sizing: border-box;
  background: linear-gradient(180deg, #ffffff 0%, #f6f8fc 100%);
  box-shadow: -12px 0 32px rgba(15, 22, 45, 0.18);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.panel-content {
  display: flex;
  flex-direction: column;
  gap: 16px;
  flex: 1;
  min-height: 0;
}

.overview {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 12px;
}

.stat-card {
  background: #f5f7fb;
  border-radius: 12px;
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.stat-label {
  font-size: 12px;
  color: #7c8295;
}

.stat-value {
  font-size: 22px;
  font-weight: 600;
  color: #1f2430;
}

.filter-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.filter-label {
  font-size: 13px;
  color: #7c8295;
}

.task-scroll {
  flex: 1;
  min-height: 0;
  padding-right: 4px;
}

.task-section + .task-section {
  margin-top: 24px;
}

.section-title {
  font-size: 16px;
  font-weight: 600;
  color: #1f2430;
  margin-bottom: 16px;
}

.fab-btn {
  position: absolute;
  bottom: 24px;
  right: 28px;
  box-shadow: 0 6px 18px rgba(0, 0, 0, 0.18);
  z-index: 500;
}
</style>
