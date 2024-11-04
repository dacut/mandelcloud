resource "aws_lambda_function" "compute_lambda" {
  architectures = ["arm64"]
  function_name = "${lower(var.project)}-${lower(var.environment)}"
  description   = "${var.project} ${var.environment} compute Lambda"
  image_uri     = data.aws_ssm_parameter.compute_lambda.value
  memory_size   = 1024
  package_type  = "Image"
  role          = aws_iam_role.compute_lambda.arn
  timeout       = 30

  environment {
    variables = {
      S3_BUCKET        = aws_s3_bucket.mandelpoints.bucket
      S3_IMAGES_PREFIX = "images/"
      S3_POINTS_PREFIX = "points/"
    }
  }

  logging_config {
    log_format = "Text"
    log_group  = aws_cloudwatch_log_group.compute_lambda.name
  }

  depends_on = [aws_iam_role_policy_attachment.compute_lambda]
}

resource "aws_lambda_function_url" "compute_lambda" {
  function_name      = aws_lambda_function.compute_lambda.function_name
  authorization_type = "NONE"
}

resource "aws_iam_role" "compute_lambda" {
  name        = "${lower(var.project)}-${lower(var.environment)}-compute-lambda"
  description = "${var.project} ${var.environment} compute lambda role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Principal = {
          Service = "lambda.amazonaws.com"
        }
        Action = "sts:AssumeRole"
      }
    ]
  })
}

data "aws_ssm_parameter" "compute_lambda" {
  name = "/${lower(var.project)}/${lower(var.environment)}/lambda-image-url"
}

resource "aws_iam_policy" "compute_lambda" {
  name        = "${lower(var.project)}-${lower(var.environment)}-compute-lambda"
  description = "${var.project} ${var.environment} compute lambda policy"
  policy      = data.aws_iam_policy_document.compute_lambda.json
}

resource "aws_iam_role_policy_attachment" "compute_lambda" {
  role       = aws_iam_role.compute_lambda.name
  policy_arn = aws_iam_policy.compute_lambda.arn
}

data "aws_iam_policy_document" "compute_lambda" {
  statement {
    effect = "Allow"
    actions = [
      "ecr:BatchGetImage",
      "ecr:DescribeImages",
      "ecr:GetDownloadUrlForLayer",
      "ecr:ListImages",
      "logs:CreateLogGroup",
      "logs:CreateLogStream",
      "logs:PutLogEvents",
      "s3:GetObject",
      "s3:PutObject",
    ]
    resources = [
      aws_ecr_repository.compute_lambda.arn,
      "${aws_cloudwatch_log_group.compute_lambda.arn}:log-stream:*",
      "${aws_s3_bucket.mandelpoints.arn}/*",
    ]
  }
}

resource "aws_cloudwatch_log_group" "compute_lambda" {
  name              = "${var.project}/${var.environment}/compute-lambda"
  log_group_class   = "INFREQUENT_ACCESS"
  retention_in_days = 7
  skip_destroy      = true
}
