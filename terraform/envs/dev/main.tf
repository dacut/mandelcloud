terraform {
  backend "s3" {}

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.69"
    }
  }
}

provider "aws" {
  region = "us-west-2"

  default_tags {
    tags = {
      Environment = "dev"
      Project     = "MandelCloud"
    }
  }
}

data "aws_partition" "current" {}
data "aws_caller_identity" "current" {}

locals {
  account_id  = data.aws_caller_identity.current.account_id
  environment = "dev"
  partition   = data.aws_partition.current.partition
  project     = "MandelCloud"
}

module "mandelcloud" {
  source = "../../modules/mandelcloud"

  account_id  = local.account_id
  environment = local.environment
  partition   = local.partition
  project     = local.project
}

output "compute_lambda_url" {
  value = module.mandelcloud.compute_lambda_url
}
