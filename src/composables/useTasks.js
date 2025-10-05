import { computed, onMounted, ref } from "vue"
import { invoke } from "@tauri-apps/api/core"

const isTauri = typeof window !== "undefined" && Boolean(window.__TAURI_INTERNALS__)

const fallbackSeeds = [
  {
    name: "产品发布检查清单",
    description: "完成上线前的最终核对事项。",
    type: "once",
    target: 5
  },
  {
    name: "周例会准备",
    description: "整理项目进展并输出会议提纲。",
    type: "cycle",
    target: 1,
    repeat: "weekly"
  },
  {
    name: "英语学习计划",
    description: "长期积累词汇并练习口语。",
    type: "long_term",
    target: 30,
    dateRange: ["2025-01-01", "2025-06-30"]
  }
]

const tasks = ref([])
const loaded = ref(false)
const loading = ref(false)
const lastError = ref(null)

const normalizeTask = (payload) => ({
  id: payload.id,
  name: payload.name,
  description: payload.description ?? "",
  type: payload.type,
  progress: payload.progress ?? 0,
  target: payload.target ?? 1,
  repeatRule: payload.repeatRule ?? null,
  startDate: payload.startDate ?? null,
  endDate: payload.endDate ?? null,
  status: payload.status ?? "active",
  createdAt: payload.createdAt ?? new Date().toISOString(),
  updatedAt: payload.updatedAt ?? new Date().toISOString()
})

const sortTasks = () => {
  tasks.value = [...tasks.value].sort((a, b) => (b.updatedAt ?? "").localeCompare(a.updatedAt ?? ""))
}

const activeTasks = computed(() => tasks.value.filter(task => task.status === "active"))
const completedTasks = computed(() => tasks.value.filter(task => task.status === "completed"))
const archivedTasks = computed(() => tasks.value.filter(task => task.status === "archived"))

const stats = computed(() => {
  const activeCount = activeTasks.value.length
  const completedCount = completedTasks.value.length
  const archivedCount = archivedTasks.value.length
  const totalVisible = activeCount + completedCount
  const completionRate = totalVisible === 0 ? 0 : Math.round((completedCount / totalVisible) * 100)

  return {
    active: activeCount,
    completed: completedCount,
    archived: archivedCount,
    completionRate
  }
})

const refreshTasks = async () => {
  if (!isTauri) {
    if (!loaded.value && tasks.value.length === 0) {
      tasks.value = fallbackSeeds.map(item => normalizeTask({ ...item, id: `${Date.now()}-${Math.random()}` }))
      sortTasks()
      loaded.value = true
    }
    return
  }

  try {
    loading.value = true
    lastError.value = null
    const response = await invoke("list_tasks", { status: null })
    tasks.value = Array.isArray(response) ? response.map(normalizeTask) : []
    sortTasks()
    loaded.value = true
  } catch (error) {
    console.error("failed to refresh tasks", error)
    lastError.value = error
  } finally {
    loading.value = false
  }
}

const ensureSeedData = async () => {
  if (!isTauri || tasks.value.length > 0) return
  for (const item of fallbackSeeds) {
    try {
      await invoke("create_task", {
        payload: {
          id: null,
          name: item.name,
          description: item.description,
          type: item.type,
          target: item.target,
          repeat: item.repeat ?? null,
          dateRange: item.dateRange ?? null
        }
      })
    } catch (error) {
      console.warn("failed to insert seed task", error)
    }
  }
  await refreshTasks()
}

const addTask = async (payload) => {
  if (!isTauri) {
    tasks.value.unshift(normalizeTask({ ...payload, id: `${Date.now()}-${Math.random()}` }))
    sortTasks()
    return tasks.value[0]
  }

  await invoke("create_task", {
    payload: {
      id: null,
      name: payload.name,
      description: payload.description ?? "",
      type: payload.type,
      target: payload.target ?? 1,
      repeat: payload.repeat ?? null,
      dateRange: payload.dateRange ?? null
    }
  })
  await refreshTasks()
  return tasks.value[0]
}

const updateTask = async (payload) => {
  if (!isTauri) {
    const index = tasks.value.findIndex(item => item.id === payload.id)
    if (index !== -1) {
      tasks.value.splice(index, 1, normalizeTask({ ...tasks.value[index], ...payload, updatedAt: new Date().toISOString() }))
      sortTasks()
      return tasks.value[index]
    }
    return null
  }

  await invoke("update_task", {
    payload: {
      id: payload.id,
      name: payload.name,
      description: payload.description ?? "",
      type: payload.type,
      target: payload.target ?? null,
      repeat: payload.repeat ?? null,
      dateRange: payload.dateRange ?? null
    }
  })
  await refreshTasks()
  return tasks.value.find(task => task.id === payload.id) ?? null
}

const deleteTask = async (id) => {
  if (!isTauri) {
    tasks.value = tasks.value.filter(task => task.id !== id)
    return
  }
  await invoke("delete_task", { payload: { id } })
  await refreshTasks()
}

const increaseProgress = async (id) => {
  if (!isTauri) {
    const task = tasks.value.find(item => item.id === id)
    if (!task) return null
    if (task.progress < task.target) {
      task.progress += 1
      if (task.progress >= task.target && task.status !== "archived") {
        task.status = "completed"
      }
      task.updatedAt = new Date().toISOString()
      sortTasks()
    }
    return task
  }

  await invoke("increase_task_progress", { payload: { id } })
  await refreshTasks()
  return tasks.value.find(task => task.id === id) ?? null
}

const archiveTask = async (id) => {
  if (!isTauri) {
    const task = tasks.value.find(item => item.id === id)
    if (task) {
      task.status = "archived"
      task.updatedAt = new Date().toISOString()
      sortTasks()
    }
    return task ?? null
  }

  await invoke("archive_task", { payload: { id } })
  await refreshTasks()
  return tasks.value.find(task => task.id === id) ?? null
}

const reopenTask = async (id) => {
  if (!isTauri) {
    const task = tasks.value.find(item => item.id === id)
    if (task) {
      task.status = "active"
      if (task.type === "cycle") {
        task.progress = 0
      }
      task.updatedAt = new Date().toISOString()
      sortTasks()
    }
    return task ?? null
  }

  await invoke("reopen_task", { payload: { id } })
  await refreshTasks()
  return tasks.value.find(task => task.id === id) ?? null
}

const importTasks = async (taskList = []) => {
  if (!Array.isArray(taskList) || taskList.length === 0) return
  for (const item of taskList) {
    try {
      await addTask(item)
    } catch (error) {
      console.warn("failed to import task", item, error)
    }
  }
  if (!isTauri) {
    sortTasks()
  }
}

const reloadTasks = async () => {
  await refreshTasks()
}

onMounted(async () => {
  await refreshTasks()
  await ensureSeedData()
})

export function useTasks() {
  return {
    tasks,
    activeTasks,
    completedTasks,
    archivedTasks,
    stats,
    loading,
    lastError,
    addTask,
    updateTask,
    deleteTask,
    increaseProgress,
    archiveTask,
    reopenTask,
    importTasks,
    reloadTasks
  }
}
