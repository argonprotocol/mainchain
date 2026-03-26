CREATE INDEX idx_notebook_status_step_notebook_number
    ON notebook_status (step, notebook_number);
