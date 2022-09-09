import * as cdk from "aws-cdk-lib";
import { Table, AttributeType, BillingMode } from "aws-cdk-lib/aws-dynamodb";
import { Construct } from "constructs";
import * as ec2 from "aws-cdk-lib/aws-ec2";
import * as lambda from "aws-cdk-lib/aws-lambda";
import * as elbv2 from "aws-cdk-lib/aws-elasticloadbalancingv2";
import * as targets from "aws-cdk-lib/aws-elasticloadbalancingv2-targets";
import { Code } from "aws-cdk-lib/aws-lambda";
import { HttpMethod, HttpApi } from "@aws-cdk/aws-apigatewayv2-alpha";
import { HttpLambdaIntegration } from "@aws-cdk/aws-apigatewayv2-integrations-alpha";
import { RemovalPolicy } from "aws-cdk-lib";
import { capitalize } from "lodash";

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

    const albApi = this.createAlbApi(sessionTable);
    const httpApi = this.createHttpApi(sessionTable);

    new cdk.CfnOutput(this, "HttpApiEndpoint", {
      value: httpApi.apiEndpoint,
      exportName: "HttpApiEndpoint",
    });
    new cdk.CfnOutput(this, "AlbApiEndpoint", {
      value: albApi.loadBalancerDnsName,
      exportName: "AlbApiEndpoint",
    });
  }

  private createAlbApi(sessionTable: Table) {
    const vpc = ec2.Vpc.fromLookup(this, "DefaultVpc", {
      vpcId: "vpc-090b0aa30d42dd996",
    });
    const alb = new elbv2.ApplicationLoadBalancer(this, "Alb", {
      vpc,
      internetFacing: true,
    });
    const httpApi = new apigwv2.HttpApi(this, "HttpApi", {
    return alb;
  }

  private createHttpApi(sessionTable: Table) {
    const fns = this.createFunctions(sessionTable, "apigw");
    const httpApi = new HttpApi(this, "HttpApi", {
      apiName: "rust-ddb-session-api",
    });

    httpApi.addRoutes({
      path: "/sessions",
      methods: [HttpMethod.GET],
      integration: new HttpLambdaIntegration("get-sessions", fns.getSessionFn),
    });

    httpApi.addRoutes({
      path: "/sessions",
      methods: [HttpMethod.POST],
      integration: new HttpLambdaIntegration(
        "create-sessions",
        fns.createSessionFn
      ),
    });

    httpApi.addRoutes({
      path: "/sessions/{username}",
      methods: [HttpMethod.DELETE],
      integration: new HttpLambdaIntegration(
        "delete-user-sessions",
        fns.deleteUserSessionsFn
      ),
    });

    return httpApi;
  }

  createFunctions(sessionTable: Table, prefix: string = "") {
    const idPrefix = capitalize(prefix);
    const namePrefix = prefix ? `${prefix}-` : "";

    const getSessionFn = new lambda.Function(this, `${idPrefix}GetSessionFn`, {
      code: Code.fromAsset("target/lambda/get-session"),
      runtime: lambda.Runtime.PROVIDED_AL2,
      handler: "bootstrap",
      functionName: `${namePrefix}rust-get-session`,
      environment: {
        TABLE_NAME: sessionTable.tableName,
        RUST_LOG: "info",
      },
    });
    sessionTable.grantReadData(getSessionFn);

    const createSessionFn = new lambda.Function(
      this,
      `${idPrefix}CreateSessionFn`,
      {
        code: Code.fromAsset("target/lambda/create-session"),
        runtime: lambda.Runtime.PROVIDED_AL2,
        handler: "bootstrap",
        functionName: `${namePrefix}rust-create-session`,
        environment: {
          TABLE_NAME: sessionTable.tableName,
          RUST_LOG: "info",
        },
      }
    );
    sessionTable.grantWriteData(createSessionFn);

    const deleteUserSessionsFn = new lambda.Function(
      this,
      `${idPrefix}DeleteUserSessionsFn`,
      {
        code: Code.fromAsset("target/lambda/delete-user-sessions"),
        runtime: lambda.Runtime.PROVIDED_AL2,
        handler: "bootstrap",
        functionName: `${namePrefix}rust-delete-user-sessions`,
        environment: {
          TABLE_NAME: sessionTable.tableName,
          RUST_LOG: "info",
        },
      }
    );
    sessionTable.grantReadWriteData(deleteUserSessionsFn);

    return {
      getSessionFn,
      createSessionFn,
      deleteUserSessionsFn,
    };
  }
}
