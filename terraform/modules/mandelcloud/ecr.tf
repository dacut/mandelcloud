resource "aws_ecr_repository" "compute_lambda" {
  name                 = "${lower(var.project)}/${lower(var.environment)}"
  image_tag_mutability = "IMMUTABLE"
  encryption_configuration {
    encryption_type = "AES256"
  }
}

resource "aws_ecr_lifecycle_policy" "compute_lambda" {
  repository = aws_ecr_repository.compute_lambda.name
  policy     = data.aws_ecr_lifecycle_policy_document.compute_lambda.json
}

data "aws_ecr_lifecycle_policy_document" "compute_lambda" {
  rule {
    priority    = 1
    description = "Keep last 5 tagged images"
    action {
      type = "expire"
    }
    selection {
      tag_status       = "tagged"
      tag_pattern_list = ["*"]
      count_type       = "imageCountMoreThan"
      count_number     = 5
    }
  }

  rule {
    priority    = 2
    description = "Delete untagged images older than 5 days"
    action {
      type = "expire"
    }
    selection {
      tag_status   = "untagged"
      count_type   = "sinceImagePushed"
      count_unit   = "days"
      count_number = 5
    }
  }
}
