use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;

#[derive(Clone)]
pub struct NoiseRemovalService {
    ffmpeg_path: String,
    temp_dir: String,
}

#[derive(Debug, Clone)]
pub struct NoiseRemovalConfig {
    pub noise_reduction: f32,
    pub sample_rate: u32,
    pub highpass_freq: u32,
    pub lowpass_freq: u32,
}

impl Default for NoiseRemovalConfig {
    fn default() -> Self {
        Self {
            noise_reduction: 0.21,
            sample_rate: 16000,
            highpass_freq: 200,
            lowpass_freq: 3000,
        }
    }
}

impl NoiseRemovalService {
    pub fn new(ffmpeg_path: Option<String>, temp_dir: Option<String>) -> Self {
        Self {
            ffmpeg_path: ffmpeg_path.unwrap_or_else(|| "ffmpeg".to_string()),
            temp_dir: temp_dir.unwrap_or_else(|| "/tmp".to_string()),
        }
    }

    pub async fn remove_noise(
        &self,
        input_path: &str,
        config: Option<NoiseRemovalConfig>,
    ) -> Result<String> {
        let start_time = std::time::Instant::now();
        let config = config.unwrap_or_default();
        let input = Path::new(input_path);
        
        if !input.exists() {
            return Err(anyhow::anyhow!("Input file not found: {}", input_path));
        }

        let output_path = self.generate_output_path(input_path).await?;
        
        tracing::info!("Starting noise removal for: {}", input_path);
        tracing::debug!("Output will be saved to: {}", output_path);

        let output = Command::new(&self.ffmpeg_path)
            .arg("-i")
            .arg(input_path)
            .arg("-af")
            .arg(self.build_filter_chain(&config))
            .arg("-ar")
            .arg(config.sample_rate.to_string())
            .arg("-ac")
            .arg("1")
            .arg("-y")
            .arg(&output_path)
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to execute FFmpeg: {}. Make sure FFmpeg is installed.", e))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            tracing::error!("FFmpeg noise removal failed: {}", error);
            return Err(anyhow::anyhow!("Noise removal failed: {}", error));
        }

        let duration = start_time.elapsed();
        tracing::info!("Noise removal completed successfully: {} (duration: {:.2}s)", output_path, duration.as_secs_f64());
        Ok(output_path)
    }

    #[allow(dead_code)]
    pub async fn remove_noise_advanced(
        &self,
        input_path: &str,
        noise_profile_duration: f32,
    ) -> Result<String> {
        let input = Path::new(input_path);
        
        if !input.exists() {
            return Err(anyhow::anyhow!("Input file not found: {}", input_path));
        }

        let noise_profile_path = self.generate_noise_profile_path(input_path).await?;
        let output_path = self.generate_output_path(input_path).await?;
        
        tracing::info!("Creating noise profile from first {} seconds", noise_profile_duration);
        
        let _profile_output = Command::new(&self.ffmpeg_path)
            .arg("-i")
            .arg(input_path)
            .arg("-t")
            .arg(noise_profile_duration.to_string())
            .arg("-af")
            .arg("arnndn=m=/usr/local/share/rnnoise/rnnoise.rnnn")
            .arg("-f")
            .arg("null")
            .arg("-")
            .output();

        let output = Command::new(&self.ffmpeg_path)
            .arg("-i")
            .arg(input_path)
            .arg("-af")
            .arg(format!(
                "highpass=f=200,lowpass=f=3000,afftdn=nf=-25,arnndn=m=/usr/local/share/rnnoise/rnnoise.rnnn:mix=0.8,volume=2.0,loudnorm=I=-16:TP=-1.5:LRA=11"
            ))
            .arg("-ar")
            .arg("16000")
            .arg("-ac")
            .arg("1")
            .arg("-y")
            .arg(&output_path)
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to execute FFmpeg: {}", e))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            tracing::warn!("Advanced noise removal failed, falling back to basic: {}", error);
            return self.remove_noise(input_path, None).await;
        }

        let _ = fs::remove_file(&noise_profile_path).await;
        
        tracing::info!("Advanced noise removal completed: {}", output_path);
        Ok(output_path)
    }

    #[allow(dead_code)]
    pub async fn cleanup_temp_file(&self, file_path: &str) -> Result<()> {
        if file_path.starts_with(&self.temp_dir) && Path::new(file_path).exists() {
            fs::remove_file(file_path).await?;
            tracing::debug!("Cleaned up temporary file: {}", file_path);
        }
        Ok(())
    }

    fn build_filter_chain(&self, config: &NoiseRemovalConfig) -> String {
        let use_loudnorm = std::env::var("NOISE_REMOVAL_USE_LOUDNORM")
            .unwrap_or_else(|_| "false".to_string())
            .to_lowercase() == "true";
        
        if use_loudnorm {
            // Slower but higher quality: includes 2-pass loudnorm
            format!(
                "highpass=f={},lowpass=f={},afftdn=nf=-{},volume=1.5,loudnorm=I=-16:TP=-1.5:LRA=11",
                config.highpass_freq,
                config.lowpass_freq,
                (config.noise_reduction * 100.0) as i32
            )
        } else {
            // Faster: skip loudnorm, use simple volume normalization
            format!(
                "highpass=f={},lowpass=f={},afftdn=nf=-{},volume=2.0",
                config.highpass_freq,
                config.lowpass_freq,
                (config.noise_reduction * 100.0) as i32
            )
        }
    }

    async fn generate_output_path(&self, input_path: &str) -> Result<String> {
        let input = Path::new(input_path);
        let filename = input
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;
        
        let extension = input
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("wav");

        let timestamp = chrono::Utc::now().timestamp_millis();
        let output_filename = format!("{}_cleaned_{}.{}", filename, timestamp, extension);
        
        let output_path = PathBuf::from(&self.temp_dir)
            .join(output_filename)
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to create output path"))?
            .to_string();

        fs::create_dir_all(&self.temp_dir).await?;
        
        Ok(output_path)
    }

    #[allow(dead_code)]
    async fn generate_noise_profile_path(&self, input_path: &str) -> Result<String> {
        let input = Path::new(input_path);
        let filename = input
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?;
        
        let timestamp = chrono::Utc::now().timestamp_millis();
        let profile_filename = format!("{}_noise_profile_{}.prof", filename, timestamp);
        
        let profile_path = PathBuf::from(&self.temp_dir)
            .join(profile_filename)
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to create profile path"))?
            .to_string();
        
        Ok(profile_path)
    }

    #[allow(dead_code)]
    pub fn is_ffmpeg_available(&self) -> bool {
        Command::new(&self.ffmpeg_path)
            .arg("-version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NoiseRemovalConfig::default();
        assert_eq!(config.noise_reduction, 0.21);
        assert_eq!(config.sample_rate, 16000);
        assert_eq!(config.highpass_freq, 200);
        assert_eq!(config.lowpass_freq, 3000);
    }

    #[test]
    fn test_filter_chain_generation() {
        let service = NoiseRemovalService::new(None, None);
        let config = NoiseRemovalConfig::default();
        let filter = service.build_filter_chain(&config);
        
        assert!(filter.contains("highpass=f=200"));
        assert!(filter.contains("lowpass=f=3000"));
        assert!(filter.contains("afftdn"));
    }

    #[tokio::test]
    async fn test_ffmpeg_availability() {
        let service = NoiseRemovalService::new(None, None);
        let available = service.is_ffmpeg_available();
        println!("FFmpeg available: {}", available);
    }
}
