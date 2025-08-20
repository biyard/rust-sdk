import * as cdk from "aws-cdk-lib";
import { Construct } from "constructs";

import "dotenv/config";
import * as rds from "aws-cdk-lib/aws-rds";
import * as cloudfront from "aws-cdk-lib/aws-cloudfront";
import * as origins from "aws-cdk-lib/aws-cloudfront-origins";
import * as s3 from "aws-cdk-lib/aws-s3";
import * as acm from "aws-cdk-lib/aws-certificatemanager";
import * as route53 from "aws-cdk-lib/aws-route53";
import * as targets from "aws-cdk-lib/aws-route53-targets";
import * as lambda from "aws-cdk-lib/aws-lambda";
import * as apigateway from "aws-cdk-lib/aws-apigateway";
import * as dynamodb from "aws-cdk-lib/aws-dynamodb";
import * as ec2 from "aws-cdk-lib/aws-ec2";
import * as ecs from "aws-cdk-lib/aws-ecs";
import * as opensearchserverless from "aws-cdk-lib/aws-opensearchserverless";
import * as cloudfrontOrigins from "aws-cdk-lib/aws-cloudfront-origins";
import * as certificatemanager from "aws-cdk-lib/aws-certificatemanager";
import * as ecs_patterns from "aws-cdk-lib/aws-ecs-patterns";
import * as iam from "aws-cdk-lib/aws-iam";
import * as elbv2 from "aws-cdk-lib/aws-elasticloadbalancingv2";
import * as event_targets from "aws-cdk-lib/aws-events-targets";
import * as events from "aws-cdk-lib/aws-events";
import * as sqs from "aws-cdk-lib/aws-sqs";
import * as lambdaEventSources from "aws-cdk-lib/aws-lambda-event-sources";

import {
  AwsLogDriver,
  Compatibility,
  ContainerImage,
  FargateService,
  TaskDefinition,
} from "aws-cdk-lib/aws-ecs";
import { Repository } from "aws-cdk-lib/aws-ecr";
import { Duration } from "aws-cdk-lib";

export class CdkStack extends cdk.Stack {
  constructor(scope: Construct, id: string, props?: cdk.StackProps) {
    super(scope, id, props);

    let project = process.env.PROJECT || "";
    let env = process.env.ENV || "";
    let domain = process.env.DOMAIN || "";
    let tableName = process.env.TABLE_NAME || "";
    let codePath = process.env.CODE_PATH || "";
    let indexes = [
      {
        name: "type-index",
        partitionKey: "type",
        sortKey: "created_at",
      },
      {
        name: "gsi1-index",
        partitionKey: "gsi1",
        sortKey: "created_at",
      },
      {
        name: "gsi2-index",
        partitionKey: "gsi2",
        sortKey: "created_at",
      },
    ];
    let enableDyanmo = process.env.ENABLE_DYNAMO === "true";
    let enableS3 = process.env.ENABLE_S3 === "true";
    let enableLambda = process.env.ENABLE_LAMBDA === "true";
    let enableFargate = process.env.ENABLE_FARGATE === "true";
    let enableOpensearch = process.env.ENABLE_OPENSEARCH === "true";
    let enableRds = process.env.ENABLE_RDS === "true";
    let createRds = process.env.CREATE_RDS === "true";
    let enableCron = process.env.ENABLE_CRON === "true";
    let enableCdn = process.env.ENABLE_CDN !== "false";
    let enableSQSWorker = process.env.ENABLE_SQS_WORKER === "true";
    let containerPort = Number(process.env.CONTAINER_PORT || "3000");
    let desiredCount = Number(process.env.DESIRED_COUNT || "1");
    let enableService = process.env.ENABLE_SERVICE === "false";
    let maxHealthyPercent = Number(process.env.MAX_HEALTHY_PERCENT || "100");
    let minHealthyPercent = Number(process.env.MIN_HEALTHY_PERCENT || "0");
    let maxCapacity = Number(process.env.MAX_CAPACITY || "20");
    let minCapacity = Number(process.env.MIN_CAPACITY || "2");
    let containerEnvs = process.env.CONTAINER_ENVS?.split(",");
    let healthPath = process.env.HEALTH_PATH || "/version";

    if (enableService) {
      maxHealthyPercent = 200;
      minHealthyPercent = 50;
    }

    let opensearchCollections = [
      {
        name: `${project}-${env}`,
        type: "SEARCH",
        description:
          "It saves and searches NFTs, Agits, Collections, DAOs, public proposal and other public data",
      },
    ];

    let vpcId = process.env.VPC_ID || "";
    const repoName = process.env.REPO_NAME || "";
    const commit = process.env.COMMIT || "";
    const registry = process.env.REGISTRY || undefined;
    const prefix = `${process.env.SERVICE}-${process.env.ENV}`;
    const versions = (process.env.VERSIONS || "").split(",");
    let endpoints = [];

    for (let version of versions) {
      if (version === "") {
        continue;
      }
      let endpoint = process.env[`${version.toUpperCase()}_ENDPOINT`];
      if (endpoint === undefined) {
        console.error(`${version.toUpperCase()}_ENDPOINT is required`);
        process.exit(1);
      }
      endpoints.push({ version, endpoint });
    }

    const hostedZoneDomainName = process.env.BASE_DOMAIN || "";
    const hostzedZoneId = process.env.HOSTED_ZONE_ID || "";

    var hostedZone: any;
    let func: any;
    var certificate: any;
    var distributionProps: any;

    if (enableCdn) {
      if (hostzedZoneId !== "") {
        hostedZone = route53.HostedZone.fromHostedZoneAttributes(
          this,
          "HostedZone",
          {
            hostedZoneId: hostzedZoneId,
            zoneName: hostedZoneDomainName,
          }
        );
      } else {
        hostedZone = route53.HostedZone.fromLookup(this, "HostedZone", {
          domainName: hostedZoneDomainName,
        });
      }

      certificate = new certificatemanager.DnsValidatedCertificate(
        this,
        "SiteCertificate",
        {
          domainName: domain,
          hostedZone,
          region: "us-east-1",
        }
      );

      distributionProps = {
        defaultBehavior: {
          origin: new origins.HttpOrigin(""),
          cachePolicy: cloudfront.CachePolicy.CACHING_DISABLED,
          allowedMethods: cloudfront.AllowedMethods.ALLOW_ALL,
          cachedMethods: cloudfront.CachedMethods.CACHE_GET_HEAD_OPTIONS,
          originRequestPolicy:
            cloudfront.OriginRequestPolicy.ALL_VIEWER_EXCEPT_HOST_HEADER,
          compress: false,
        },
        domainNames: [domain],
        certificate,
      };
    }

    if (enableSQSWorker) {
      func = new lambda.Function(this, "Function", {
        runtime: lambda.Runtime.PROVIDED_AL2023,
        code: lambda.Code.fromAsset(codePath),
        handler: "bootstrap",
        environment: {
          NO_COLOR: "true",
        },
        memorySize: 512,
        timeout: cdk.Duration.seconds(30),
      });

      const queue = new sqs.Queue(this, "Queue", {
        visibilityTimeout: cdk.Duration.seconds(300),
        retentionPeriod: cdk.Duration.days(14),
        fifo: false,
      });

      func.addEventSource(
        new lambdaEventSources.SqsEventSource(queue, {
          batchSize: 10,
          maxBatchingWindow: cdk.Duration.seconds(5),
          reportBatchItemFailures: true,
        })
      );

      queue.grantConsumeMessages(func);
    }

    if (enableLambda) {
      func = new lambda.Function(this, "Function", {
        runtime: lambda.Runtime.PROVIDED_AL2023,
        code: lambda.Code.fromAsset(codePath),
        handler: "bootstrap",
        environment: {
          NO_COLOR: "true",
        },
        memorySize: 512,
        timeout: cdk.Duration.seconds(30),
      });

      const api = new apigateway.LambdaRestApi(this, "Api", {
        handler: func,
        proxy: true,
      });

      distributionProps.defaultBehavior.origin = new origins.RestApiOrigin(api);

      if (enableCron) {
        const schedule = process.env.SCHEDULE || "";

        if (schedule === "") {
          console.error(
            "SCHEDULE is required ex. SCHEDULE='cron(0 15 * * ? *)'"
          );
          process.exit(1);
        }

        const rule = new events.Rule(this, "ScheduleRule", {
          schedule: events.Schedule.expression(schedule), //KST 00:00
        });

        rule.addTarget(new event_targets.LambdaFunction(func));
      }
    }

    if (enableDyanmo) {
      const table = new dynamodb.Table(this, "DynamoDB", {
        partitionKey: {
          name: "id",
          type: dynamodb.AttributeType.STRING,
        },
        tableName,
        removalPolicy: cdk.RemovalPolicy.RETAIN,
        billingMode: dynamodb.BillingMode.PAY_PER_REQUEST,
      });

      for (let index of indexes) {
        table.addGlobalSecondaryIndex({
          indexName: index.name,
          partitionKey: {
            name: index.partitionKey,
            type: dynamodb.AttributeType.STRING,
          },
          sortKey: {
            name: index.sortKey,
            type: dynamodb.AttributeType.NUMBER,
          },
          projectionType: dynamodb.ProjectionType.ALL,
        });
      }

      if (enableLambda) {
        table.grantReadWriteData(func);
      }
    } else if (!enableDyanmo && tableName !== "") {
      const table = dynamodb.Table.fromTableName(this, "DynamoDB", tableName);
      if (enableLambda) {
        table.grantReadWriteData(func);
      }
    }

    if (enableS3) {
      var allowedOrigins = [
        `https://${domain}`,
        `https://dev.${domain}`,
        `https://stage.${domain}`,
        "http://localhost:*",
      ];

      const assetsBucket = new s3.Bucket(this, "Bucket", {
        bucketName: domain,
        removalPolicy: cdk.RemovalPolicy.DESTROY,
        cors: [
          {
            allowedHeaders: ["*"],
            allowedMethods: [s3.HttpMethods.PUT, s3.HttpMethods.POST],
            allowedOrigins,
            exposedHeaders: [],
          },
        ],
      });

      const s3Origin = new origins.S3Origin(assetsBucket);

      if (!enableLambda && !enableFargate) {
        distributionProps.defaultBehavior.origin = s3Origin;
        distributionProps.defaultBehavior.cachePolicy =
          cloudfront.CachePolicy.CACHING_OPTIMIZED;
        distributionProps.defaultBehavior.compress = true;
      } else {
        distributionProps.additionalBehaviors = {
          "/metadata/*": {
            origin: s3Origin,
            cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
            compress: true,
          },
          "/assets/*": {
            origin: s3Origin,
            cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
            compress: true,
          },
          "/*.js": {
            origin: s3Origin,
            cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
            compress: true,
          },
          "/*.css": {
            origin: s3Origin,
            cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
            compress: true,
          },
          "/*.html": {
            origin: s3Origin,
            cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
            compress: true,
          },
          "/*.ico": {
            origin: s3Origin,
            cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
            compress: true,
          },
          "/*.svg": {
            origin: s3Origin,
            cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
            compress: true,
          },
          "/*.avif": {
            origin: s3Origin,
            cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
            compress: true,
          },
          "/*.png": {
            origin: s3Origin,
            cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
            compress: true,
          },
          "/*.wasm": {
            origin: s3Origin,
            cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
            compress: true,
          },
          "/icons/*": {
            origin: s3Origin,
            cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
            compress: true,
          },
          "/images/*": {
            origin: s3Origin,
            cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
            compress: true,
          },
          "/public/*": {
            origin: s3Origin,
            cachePolicy: cloudfront.CachePolicy.CACHING_OPTIMIZED,
            compress: true,
          },
        };
      }
    }

    for (let endpoint of endpoints) {
      let path = `/${endpoint.version}/*`;
      distributionProps.additionalBehaviors[path] = {
        origin: new origins.HttpOrigin(endpoint.endpoint),
        cachePolicy: cloudfront.CachePolicy.CACHING_DISABLED,
        allowedMethods: cloudfront.AllowedMethods.ALLOW_ALL,
        viewerProtocolPolicy: cloudfront.ViewerProtocolPolicy.REDIRECT_TO_HTTPS,
        originRequestPolicy: cloudfront.OriginRequestPolicy.ALL_VIEWER,
        compress: false,
      };
    }

    if (enableFargate) {
      const vpc = ec2.Vpc.fromLookup(this, "Vpc", {
        vpcId,
      });

      const cluster = new ecs.Cluster(this, "Cluster", {
        vpc,
      });

      const taskExecutionRole = new iam.Role(this, "MyEcsTaskExecutionRole", {
        assumedBy: new iam.ServicePrincipal("ecs-tasks.amazonaws.com"),
      });

      taskExecutionRole.addManagedPolicy(
        iam.ManagedPolicy.fromAwsManagedPolicyName(
          "service-role/AmazonECSTaskExecutionRolePolicy"
        )
      );

      taskExecutionRole.addToPolicy(
        new iam.PolicyStatement({
          actions: [
            "ecr:GetAuthorizationToken",
            "ecr:BatchCheckLayerAvailability",
            "ecr:GetDownloadUrlForLayer",
            "ecr:BatchGetImage",
          ],
          resources: ["*"],
        })
      );

      const taskDefinition = new TaskDefinition(this, "FargateTask", {
        compatibility: Compatibility.FARGATE,
        cpu: "256",
        memoryMiB: "512",
        executionRole: taskExecutionRole,
      });

      let options: any = {
        logging: new AwsLogDriver({
          streamPrefix: `${prefix}-logs`,
        }),
        environment: {},
      };

      if (registry) {
        options.image = ContainerImage.fromRegistry(registry);
      } else {
        const repository = Repository.fromRepositoryName(
          this,
          "FetcherRepository",
          repoName
        );

        options.image = ContainerImage.fromEcrRepository(repository, commit);
      }

      for (let env of containerEnvs || []) {
        options.environment[env] = process.env[env];
      }

      const container = taskDefinition.addContainer("Container", options);

      container.addPortMappings({
        containerPort,
      });

      const service = new FargateService(this, "FargateService", {
        cluster,
        taskDefinition,
        desiredCount,
        maxHealthyPercent,
        minHealthyPercent,
      });

      if (enableService) {
        const scaling = service.autoScaleTaskCount({
          minCapacity,
          maxCapacity,
        });
        scaling.scaleOnCpuUtilization("CpuScaling", {
          targetUtilizationPercent: 50,
          scaleInCooldown: cdk.Duration.seconds(60),
          scaleOutCooldown: cdk.Duration.seconds(60),
        });
        scaling.scaleOnMemoryUtilization("MemoryScaling", {
          targetUtilizationPercent: 50,
          scaleInCooldown: cdk.Duration.seconds(60),
          scaleOutCooldown: cdk.Duration.seconds(60),
        });
      }

      const alb = new elbv2.ApplicationLoadBalancer(this, "ALB", {
        vpc,
        internetFacing: true,
      });

      const listener = alb.addListener("HttpListener", {
        port: 80,
      });
      const targetGroup = new elbv2.ApplicationTargetGroup(
        this,
        "TargetGroup",
        {
          targets: [service],
          protocol: elbv2.ApplicationProtocol.HTTP,
          vpc: vpc,
          port: 80,
          deregistrationDelay: cdk.Duration.seconds(30),
          healthCheck: {
            path: healthPath,
            healthyThresholdCount: 2,
            unhealthyThresholdCount: 3,
            interval: cdk.Duration.seconds(10),
            timeout: cdk.Duration.seconds(5),
            healthyHttpCodes: "200",
          },
        }
      );

      listener.addAction("HttpDefaultAction", {
        action: elbv2.ListenerAction.forward([targetGroup]),
      });

      distributionProps.defaultBehavior.origin =
        new cloudfrontOrigins.LoadBalancerV2Origin(alb, {
          protocolPolicy: cloudfront.OriginProtocolPolicy.HTTP_ONLY,
        });
    }

    if (enableOpensearch) {
      let collections = [];
      let i = 0;
      for (let collection of opensearchCollections) {
        const c = new opensearchserverless.CfnCollection(
          this,
          `${collection.name}-collection`,
          collection
        );

        const encPolicy = new opensearchserverless.CfnSecurityPolicy(
          this,
          `SecurityPolicy-${collection.name}`,
          {
            name: `${collection.name}-collection-policy`,
            policy: `{"Rules":[{"ResourceType":"collection","Resource":["collection/${collection.name}"]}],"AWSOwnedKey":true}`,
            type: "encryption",
          }
        );
        c.addDependency(encPolicy);

        // Network policy is required so that the dashboard can be viewed!
        const netPolicy = new opensearchserverless.CfnSecurityPolicy(
          this,
          `NetworkPolicy-${collection.name}`,
          {
            name: `${collection.name}-network-policy`,
            policy: `[{"Rules":[{"ResourceType":"collection","Resource":["collection/${collection.name}"]}, {"ResourceType":"dashboard","Resource":["collection/${collection.name}"]}],"AllowFromPublic":true}]`,
            type: "network",
          }
        );
        c.addDependency(netPolicy);

        new cdk.CfnOutput(this, `OpenSearchServerlessEndpoint${i}`, {
          value: `https://${c.attrCollectionEndpoint}`,
          description: "The endpoint of the OpenSearch Serverless collection",
        });

        collections.push(c.attrArn);
      }

      if (enableLambda) {
        func.addToRolePolicy(
          new iam.PolicyStatement({
            actions: ["aoss:*"],
            resources: collections,
          })
        );
      }
    }

    if (createRds) {
      const adminPassword = process.env.RDS_ADMIN_PASSWORD || "";
      if (adminPassword === "") {
        console.error("RDS_ADMIN_PASSWORD is required");
        process.exit(1);
      }

      const vpc = ec2.Vpc.fromLookup(this, "Vpc", {
        vpcId,
      });
      const securityGroup = new ec2.SecurityGroup(this, "AuroraSecurityGroup", {
        vpc,
        allowAllOutbound: true,
      });
      securityGroup.addIngressRule(
        ec2.Peer.anyIpv4(),
        ec2.Port.tcp(5432),
        "Allow PostgreSQL access from anywhere"
      );

      const cluster = new rds.DatabaseCluster(this, "Database", {
        engine: rds.DatabaseClusterEngine.auroraPostgres({
          version: rds.AuroraPostgresEngineVersion.VER_16_4,
        }),
        writer: rds.ClusterInstance.serverlessV2("writer"),
        // readers: [
        //   rds.ClusterInstance.serverlessV2('reader'),
        // ],
        serverlessV2MinCapacity: 0.5,
        serverlessV2MaxCapacity: 256,
        vpc,
        vpcSubnets: { subnetType: ec2.SubnetType.PUBLIC },
        removalPolicy: cdk.RemovalPolicy.RETAIN,
        securityGroups: [securityGroup],
        defaultDatabaseName: `${project}`,
        credentials: rds.Credentials.fromPassword(
          project,
          cdk.SecretValue.unsafePlainText(adminPassword)
        ),
        deletionProtection: true,
      });

      cluster
        .metricServerlessDatabaseCapacity({
          period: Duration.minutes(10),
        })
        .createAlarm(this, "capacity", {
          threshold: 1.5,
          evaluationPeriods: 3,
        });
      cluster
        .metricACUUtilization({
          period: Duration.minutes(10),
        })
        .createAlarm(this, "alarm", {
          evaluationPeriods: 3,
          threshold: 90,
        });

      new cdk.CfnOutput(this, "AuroraEndpoint", {
        value: cluster.clusterEndpoint.hostname,
        description: "The endpoint of the Aurora PostgreSQL cluster",
      });
    }

    if (!createRds && enableRds) {
      console.error(
        "creating an individual db cluster for service is not recommended. Instead of it, you manually create a table in `ENV` database cluster. If you want to create a new individual db cluster, set CREATE_RDS=true"
      );
      process.exit(1);

      const adminPassword = process.env.RDS_ADMIN_PASSWORD || "";
      if (adminPassword === "") {
        console.error("RDS_ADMIN_PASSWORD is required");
        process.exit(1);
      }

      const vpc = ec2.Vpc.fromLookup(this, "Vpc", {
        vpcId,
      });
      const securityGroup = new ec2.SecurityGroup(this, "AuroraSecurityGroup", {
        vpc,
        allowAllOutbound: true,
      });
      securityGroup.addIngressRule(
        ec2.Peer.anyIpv4(),
        ec2.Port.tcp(5432),
        "Allow PostgreSQL access from anywhere"
      );

      const cluster = new rds.DatabaseCluster(this, "Database", {
        engine: rds.DatabaseClusterEngine.auroraPostgres({
          version: rds.AuroraPostgresEngineVersion.VER_16_4,
        }),
        writer: rds.ClusterInstance.serverlessV2("writer"),
        // readers: [
        //   rds.ClusterInstance.serverlessV2('reader'),
        // ],
        serverlessV2MinCapacity: 0.5,
        serverlessV2MaxCapacity: 256,
        vpc,
        vpcSubnets: { subnetType: ec2.SubnetType.PUBLIC },
        removalPolicy: cdk.RemovalPolicy.RETAIN,
        securityGroups: [securityGroup],
        defaultDatabaseName: `${project}`.replace("-", "").replace("_", ""),
        credentials: rds.Credentials.fromPassword(
          project.replace("-", "").replace("_", ""),
          cdk.SecretValue.unsafePlainText(adminPassword)
        ),
        deletionProtection: true,
      });

      cluster
        .metricServerlessDatabaseCapacity({
          period: Duration.minutes(10),
        })
        .createAlarm(this, "capacity", {
          threshold: 1.5,
          evaluationPeriods: 3,
        });
      cluster
        .metricACUUtilization({
          period: Duration.minutes(10),
        })
        .createAlarm(this, "alarm", {
          evaluationPeriods: 3,
          threshold: 90,
        });

      new cdk.CfnOutput(this, "AuroraEndpoint", {
        value: cluster.clusterEndpoint.hostname,
        description: "The endpoint of the Aurora PostgreSQL cluster",
      });
    }

    if (enableCdn) {
      const cf = new cloudfront.Distribution(
        this,
        "Distribution",
        distributionProps
      );

      const zone = route53.HostedZone.fromHostedZoneAttributes(
        this,
        "zone-attribute",
        {
          zoneName: domain,
          hostedZoneId: hostedZone.hostedZoneId,
        }
      );

      new route53.ARecord(this, "IpV4Record", {
        zone,
        target: route53.RecordTarget.fromAlias(
          new targets.CloudFrontTarget(cf)
        ),
      });

      new route53.AaaaRecord(this, "IpV6Record", {
        zone,
        target: route53.RecordTarget.fromAlias(
          new targets.CloudFrontTarget(cf)
        ),
      });
    }
  }
}
