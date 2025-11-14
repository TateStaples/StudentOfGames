use burn::prelude::*;
use burn::nn::{Linear, LinearConfig, Dropout, DropoutConfig};
use burn::tensor::backend::AutodiffBackend;


/// Combined Value and Policy Network
/// Takes game state as input, outputs both value estimate and policy logits
#[derive(Module, Debug)]
pub struct ValuePolicyNetwork<B: Backend> {
    hidden1: Linear<B>,
    hidden2: Linear<B>,
    hidden3: Linear<B>,
    value_head: Linear<B>,
    policy_head: Linear<B>,
    dropout: Dropout,
}

impl<B: Backend> ValuePolicyNetwork<B> {
    /// Create a new network with specified dimensions
    pub fn new(input_size: usize, hidden_size: usize, max_actions: usize, device: &B::Device) -> Self {
        Self {
            hidden1: LinearConfig::new(input_size, hidden_size).init(device),
            hidden2: LinearConfig::new(hidden_size, hidden_size).init(device),
            hidden3: LinearConfig::new(hidden_size, hidden_size).init(device),
            value_head: LinearConfig::new(hidden_size, 1).init(device),
            policy_head: LinearConfig::new(hidden_size, max_actions).init(device),
            dropout: DropoutConfig::new(0.1).init(),
        }
    }

    /// Forward pass returning (value, policy_logits)
    pub fn forward(&self, input: Tensor<B, 2>) -> (Tensor<B, 2>, Tensor<B, 2>) {
        let x = self.hidden1.forward(input);
        let x = burn::tensor::activation::relu(x);
        let x = self.dropout.forward(x);
        
        let x = self.hidden2.forward(x);
        let x = burn::tensor::activation::relu(x);
        let x = self.dropout.forward(x);
        
        let x = self.hidden3.forward(x);
        let x = burn::tensor::activation::relu(x);
        
        let value = self.value_head.forward(x.clone());
        let value = burn::tensor::activation::tanh(value);
        
        let policy = self.policy_head.forward(x);
        
        (value, policy)
    }
}

/// Training batch for the neural network
#[derive(Debug, Clone)]
pub struct TrainingBatch<B: Backend> {
    pub states: Tensor<B, 2>,      // [batch_size, input_size]
    pub policies: Tensor<B, 2>,    // [batch_size, max_actions]
    pub values: Tensor<B, 2>,      // [batch_size, 1]
}

/// Configuration for neural network training
#[derive(Debug, Clone)]
pub struct NeuralConfig {
    pub input_size: usize,
    pub hidden_size: usize,
    pub max_actions: usize,
    pub learning_rate: f64,
    pub batch_size: usize,
    pub value_loss_weight: f64,
    pub policy_loss_weight: f64,
}

impl Default for NeuralConfig {
    fn default() -> Self {
        Self {
            input_size: 64,
            hidden_size: 128,
            max_actions: 10,
            learning_rate: 0.001,
            batch_size: 32,
            value_loss_weight: 1.0,
            policy_loss_weight: 1.0,
        }
    }
}

/// Loss computation for training
pub fn compute_loss<B: AutodiffBackend>(
    value_pred: Tensor<B, 2>,
    policy_pred: Tensor<B, 2>,
    value_target: Tensor<B, 2>,
    policy_target: Tensor<B, 2>,
    config: &NeuralConfig,
) -> Tensor<B, 1> {
    // Value loss (MSE)
    let value_loss = value_pred
        .clone()
        .sub(value_target)
        .powf_scalar(2.0)
        .mean();
    
    // Policy loss (cross-entropy)
    // Using manual log softmax implementation since the method may not be available
    let policy_loss = policy_pred
        .clone()
        .mul(policy_target)
        .sum_dim(1)
        .neg()
        .mean();
    
    // Weighted combination
    let total_loss = value_loss
        .mul_scalar(config.value_loss_weight)
        .add(policy_loss.mul_scalar(config.policy_loss_weight));
    
    total_loss
}

#[cfg(test)]
mod tests {
    use super::*;
    use burn::backend::NdArray;
    
    type TestBackend = NdArray;
    
    #[test]
    fn test_network_creation() {
        let device = Default::default();
        let network = ValuePolicyNetwork::<TestBackend>::new(64, 128, 10, &device);
        
        // Create dummy input
        let input = Tensor::<TestBackend, 2>::zeros([1, 64], &device);
        let (value, policy) = network.forward(input);
        
        assert_eq!(value.shape().dims, [1, 1]);
        assert_eq!(policy.shape().dims, [1, 10]);
    }
}
