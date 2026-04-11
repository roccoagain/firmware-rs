pub struct PidFilter {
    /// Proportional gain
    pub kp: f32,
    /// Integral gain
    pub ki: f32,
    /// Derivative gain
    pub kd: f32,
    /// Feedforward gain
    pub kf: f32,

    /// Integrated error
    pub sum_error: f32,
    /// Previous error
    pub prev_error: f32,

    /// Target
    pub setpoint: f32,
    /// Estimate
    pub measurement: f32,

    /// Whether to wrap error value
    pub wrap: bool,
    /// Wrapping min value
    pub wrap_min: f32,
    /// Wrapping max value
    pub wrap_max: f32,
}

impl PidFilter {
    pub fn new() -> Self {
        Self {
            kp: 0.0,
            ki: 0.0,
            kd: 0.0,
            kf: 0.0,
            sum_error: 0.0,
            prev_error: 0.0,
            setpoint: 0.0,
            measurement: 0.0,
            wrap: false,
            wrap_min: 0.0,
            wrap_max: 0.0,
        }
    }

    /// Calculate PIDF output.
    pub fn filter(&mut self, dt: f32, bound: bool, wrap: bool) -> f32 {
        let mut error = self.setpoint - self.measurement;

        if error > core::f32::consts::PI && wrap {
            error -= 2.0 * core::f32::consts::PI;
        }
        if error < -core::f32::consts::PI && wrap {
            error += 2.0 * core::f32::consts::PI;
        }

        self.sum_error += error * dt;

        let mut output = (self.kp * error) + (self.kd * ((error - self.prev_error) / dt)) + self.kf;

        self.prev_error = error;

        if output.abs() > 1.0 && bound {
            output /= output.abs();
        }

        output
    }

    /// Set the PIDF gains.
    pub fn set_gains(&mut self, kp: f32, ki: f32, kd: f32, kf: f32) {
        self.kp = kp;
        self.ki = ki;
        self.kd = kd;
        self.kf = kf;
    }
}
