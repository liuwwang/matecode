use super::{Plan, PlanStatus};
use crate::config;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

/// 计划存储管理器
pub struct PlanStorage {
    storage_dir: PathBuf,
}

/// 存储的计划信息
#[derive(Debug, Serialize, Deserialize)]
pub struct StoredPlan {
    pub plan: Plan,
    pub current_step: usize,
    pub completed_steps: Vec<usize>,
    pub failed_steps: Vec<usize>,
}

impl PlanStorage {
    pub async fn new() -> Result<Self> {
        let config_dir = config::get_config_dir().await?;
        let storage_dir = config_dir.join("plans");
        
        // 确保存储目录存在
        if !storage_dir.exists() {
            fs::create_dir_all(&storage_dir).await?;
        }
        
        Ok(Self { storage_dir })
    }
    
    /// 保存计划
    pub async fn save_plan(&self, plan: &Plan) -> Result<()> {
        let stored_plan = StoredPlan {
            plan: plan.clone(),
            current_step: 0,
            completed_steps: vec![],
            failed_steps: vec![],
        };
        
        let file_path = self.get_plan_file_path(&plan.id);
        let content = serde_json::to_string_pretty(&stored_plan)?;
        fs::write(file_path, content).await?;
        
        // 同时保存为当前活动计划
        self.save_as_current_plan(plan).await?;
        
        Ok(())
    }
    
    /// 加载计划
    pub async fn load_plan(&self, plan_id: &str) -> Result<StoredPlan> {
        let file_path = self.get_plan_file_path(plan_id);
        
        if !file_path.exists() {
            return Err(anyhow!("计划不存在: {}", plan_id));
        }
        
        let content = fs::read_to_string(file_path).await?;
        let stored_plan: StoredPlan = serde_json::from_str(&content)?;
        
        Ok(stored_plan)
    }
    
    /// 更新计划执行状态
    pub async fn update_plan_progress(&self, plan_id: &str, current_step: usize, completed_steps: Vec<usize>, failed_steps: Vec<usize>) -> Result<()> {
        let mut stored_plan = self.load_plan(plan_id).await?;
        stored_plan.current_step = current_step;
        stored_plan.completed_steps = completed_steps;
        stored_plan.failed_steps = failed_steps;
        
        let file_path = self.get_plan_file_path(plan_id);
        let content = serde_json::to_string_pretty(&stored_plan)?;
        fs::write(file_path, content).await?;
        
        Ok(())
    }
    
    /// 保存为当前活动计划
    async fn save_as_current_plan(&self, plan: &Plan) -> Result<()> {
        let current_plan_file = self.storage_dir.join("current.json");
        let content = serde_json::to_string_pretty(plan)?;
        fs::write(current_plan_file, content).await?;
        Ok(())
    }
    
    /// 加载当前活动计划
    pub async fn load_current_plan(&self) -> Result<Plan> {
        let current_plan_file = self.storage_dir.join("current.json");
        
        if !current_plan_file.exists() {
            return Err(anyhow!("没有当前活动的计划"));
        }
        
        let content = fs::read_to_string(current_plan_file).await?;
        let plan: Plan = serde_json::from_str(&content)?;
        
        Ok(plan)
    }
    
    /// 删除计划
    pub async fn delete_plan(&self, plan_id: &str) -> Result<()> {
        let file_path = self.get_plan_file_path(plan_id);
        
        if file_path.exists() {
            fs::remove_file(file_path).await?;
        }
        
        Ok(())
    }
    
    /// 列出所有计划
    pub async fn list_plans(&self) -> Result<Vec<String>> {
        let mut plans = Vec::new();
        let mut entries = fs::read_dir(&self.storage_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Some(file_name) = path.file_stem() {
                    if file_name != "current" {
                        plans.push(file_name.to_string_lossy().to_string());
                    }
                }
            }
        }
        
        Ok(plans)
    }
    
    /// 获取计划文件路径
    fn get_plan_file_path(&self, plan_id: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.json", plan_id))
    }
}
