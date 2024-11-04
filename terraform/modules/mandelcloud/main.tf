variable "account_id" {
  type        = string
  description = "The AWS account ID resources are being deployed to"
}

variable "environment" {
  type        = string
  description = "The environment resources are being deployed to (dev, staging, prod, etc)"
}

variable "partition" {
  type        = string
  description = "The AWS partition resources are being deployed to"
}

variable "project" {
  type        = string
  description = "The name of the project resources are being deployed to"
}

output "compute_lambda_url" {
  value = aws_lambda_function_url.compute_lambda.function_url
}
