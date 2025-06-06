use crate::layers::HiddenAct;
use candle::{Device, Result, Tensor};
use std::sync::Once;

#[cfg(feature = "cuda")]
use candle_cublaslt::{fused_batch_matmul, fused_matmul, Activation, CublasLt};

static INIT: Once = Once::new();
static mut CUBLASLT: Option<CublasLtWrapper> = None;

pub fn get_cublas_lt_wrapper() -> Option<&'static CublasLtWrapper> {
    unsafe {
        INIT.call_once(|| {
            #[cfg(not(feature = "cuda"))]
            {
                CUBLASLT = None;
            }

            #[cfg(feature = "cuda")]
            {
                // Check if we can call the driver
                // Then check if we can create a device
                // Then check that the device is CUDA
                use candle::cuda_backend::cudarc::driver;
                CUBLASLT = driver::result::init()
                    .ok()
                    .and_then(|_| Device::cuda_if_available(0).ok())
                    .and_then(|device| match device {
                        Device::Cuda(_) => Some(CublasLtWrapper {
                            cublaslt: CublasLt::new(&device).unwrap(),
                        }),
                        _ => None,
                    });
            }
        });
        #[allow(static_mut_refs)]
        CUBLASLT.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct CublasLtWrapper {
    #[cfg(feature = "cuda")]
    pub cublaslt: CublasLt,
}

impl CublasLtWrapper {
    #[allow(clippy::too_many_arguments)]
    pub fn matmul(
        &self,
        a: &Tensor,
        b: &Tensor,
        out: Option<&Tensor>,
        alpha: Option<f32>,
        beta: Option<f32>,
        bias: Option<&Tensor>,
        act: Option<HiddenAct>,
    ) -> Result<Tensor> {
        #[cfg(feature = "cuda")]
        {
            let inner_act = match act {
                Some(HiddenAct::Gelu) => Some(Activation::Gelu),
                Some(HiddenAct::Relu) => Some(Activation::Relu),
                _ => None,
            };

            let mut result = fused_matmul(
                a,
                b,
                out,
                alpha,
                beta,
                bias,
                inner_act,
                self.cublaslt.clone(),
            )?;

            if Some(HiddenAct::Swiglu) == act {
                result = candle_nn::ops::swiglu(&result)?;
            }
            Ok(result)
        }
        #[cfg(not(feature = "cuda"))]
        {
            candle::bail!("`cuda` feature is not enabled")
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn batch_matmul(
        &self,
        a: &Tensor,
        b: &Tensor,
        out: Option<&Tensor>,
        alpha: Option<f32>,
        beta: Option<f32>,
        bias: Option<&Tensor>,
        act: Option<HiddenAct>,
    ) -> Result<Tensor> {
        #[cfg(feature = "cuda")]
        {
            let inner_act = match act {
                Some(HiddenAct::Gelu) => Some(Activation::Gelu),
                Some(HiddenAct::Relu) => Some(Activation::Relu),
                _ => None,
            };

            let mut result = fused_batch_matmul(
                a,
                b,
                out,
                alpha,
                beta,
                bias,
                inner_act,
                self.cublaslt.clone(),
            )?;

            if Some(HiddenAct::Swiglu) == act {
                result = candle_nn::ops::swiglu(&result)?;
            }
            Ok(result)
        }
        #[cfg(not(feature = "cuda"))]
        {
            candle::bail!("`cuda` feature is not enabled")
        }
    }
}
