FROM mcr.microsoft.com/dotnet/sdk:10.0 AS build
WORKDIR /src
COPY Directory.Build.props CHANGELOG.md ./
COPY Shared/ Shared/
COPY Server/ Server/
RUN dotnet publish Server/LicenseVault.Server.csproj -c Release --self-contained false -p:UseAppHost=false -o /app

FROM mcr.microsoft.com/dotnet/aspnet:10.0
WORKDIR /app
COPY --from=build /app/ ./
COPY --chmod=755 scripts/start-railway.sh /usr/local/bin/start-licentiekluis
ENV LKV_DATA=/data
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/start-licentiekluis"]
