resource "aws_s3_bucket" "mandelpoints" {
  bucket_prefix = "${lower(var.project)}-${lower(var.environment)}-"
}

resource "aws_s3_bucket_server_side_encryption_configuration" "mandelpoints" {
  bucket = aws_s3_bucket.mandelpoints.bucket
  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "mandelpoints" {
  bucket                  = aws_s3_bucket.mandelpoints.bucket
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_policy" "mandelpoints" {
  bucket = aws_s3_bucket.mandelpoints.bucket
  policy = data.aws_iam_policy_document.mandelpoints.json
}

data "aws_iam_policy_document" "mandelpoints" {
  statement {
    effect    = "Allow"
    actions   = ["s3:GetObject", "s3:PutObject"]
    resources = ["${aws_s3_bucket.mandelpoints.arn}/*"]
    principals {
      type        = "AWS"
      identifiers = ["arn:aws:iam::${var.account_id}:role/${aws_iam_role.compute_lambda.name}"]
    }
  }
}
