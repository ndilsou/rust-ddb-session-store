import * as cdk from "aws-cdk-lib";
import { Table, AttributeType, BillingMode } from "aws-cdk-lib/aws-dynamodb";
import { Construct } from "constructs";
import * as lambda from "aws-cdk-lib/aws-lambda";
import * as apigwv2 from "@aws-cdk/aws-apigatewayv2-alpha";
import { Code } from "aws-cdk-lib/aws-lambda";
import { HttpMethod } from "@aws-cdk/aws-apigatewayv2-alpha";
import { HttpLambdaIntegration } from "@aws-cdk/aws-apigatewayv2-integrations-alpha";
import { RemovalPolicy } from "aws-cdk-lib";

// import * as sqs from 'aws-cdk-lib/aws-sqs';

export class DdbSessionStoreStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    // The code that defines your stack goes here
    const sessionTable = new Table(this, "SessionTable", {
      partitionKey: { name: "PK", type: AttributeType.STRING },
      billingMode: BillingMode.PAY_PER_REQUEST,
      removalPolicy: RemovalPolicy.DESTROY,
      timeToLiveAttribute: "TTL",
    });
    sessionTable.addGlobalSecondaryIndex({
      indexName: "GSI1",
      partitionKey: { name: "GSI1PK", type: AttributeType.STRING },
    });
    const httpApi = new apigwv2.HttpApi(this, "HttpApi", {
      apiName: "rust-ddb-session-api",
    });

    const getSessionFn = new lambda.Function(this, "GetSessionFn", {
      code: Code.fromAsset("target/lambda/get-session"),
      runtime: lambda.Runtime.PROVIDED_AL2,
      handler: "bootstrap",
      functionName: "rust-get-session",
      environment: {
        TABLE_NAME: sessionTable.tableName,
        RUST_LOG: "info",
      },
    });
    sessionTable.grantReadData(getSessionFn);

    httpApi.addRoutes({
      path: "/sessions",
      methods: [HttpMethod.GET],
      integration: new HttpLambdaIntegration("get-sessions", getSessionFn),
    });

    const createSessionFn = new lambda.Function(this, "CreateSessionFn", {
      code: Code.fromAsset("target/lambda/create-session"),
      runtime: lambda.Runtime.PROVIDED_AL2,
      handler: "bootstrap",
      functionName: "rust-create-session",
      environment: {
        TABLE_NAME: sessionTable.tableName,
        RUST_LOG: "info",
      },
    });
    sessionTable.grantWriteData(createSessionFn);

    httpApi.addRoutes({
      path: "/sessions",
      methods: [HttpMethod.POST],
      integration: new HttpLambdaIntegration(
        "create-sessions",
        createSessionFn
      ),
    });

    const deleteUserSessionsFn = new lambda.Function(
      this,
      "DeleteUserSessionsFn",
      {
        code: Code.fromAsset("target/lambda/delete-user-sessions"),
        runtime: lambda.Runtime.PROVIDED_AL2,
        handler: "bootstrap",
        functionName: "rust-delete-user-sessions",
        environment: {
          TABLE_NAME: sessionTable.tableName,
          RUST_LOG: "info",
        },
      }
    );
    sessionTable.grantReadWriteData(deleteUserSessionsFn);

    httpApi.addRoutes({
      path: "/sessions/{username}",
      methods: [HttpMethod.DELETE],
      integration: new HttpLambdaIntegration(
        "delete-user-sessions",
        deleteUserSessionsFn
      ),
    });

    new cdk.CfnOutput(this, "HttpApiEndpoint", {
      value: httpApi.apiEndpoint,
      exportName: "HttpApiEndpoint",
    });

    // example resource
    // const queue = new sqs.Queue(this, 'DdbSessionStoreQueue', {
    //   visibilityTimeout: cdk.Duration.seconds(300)
    // });
  }
}
