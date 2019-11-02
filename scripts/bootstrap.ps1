#
# The script creates a bft or genesis configuration with 1 faucet with a hardcoded amount
# It is based on the bootstrap shell script at https://github.com/input-output-hk/jormungandr/tree/master/scripts
#
# Disclaimer:
#
#  The following use of Powershell script is for demonstration and understanding
#  only, it should *NOT* be used at scale or for any sort of serious
#  deployment, and is solely used for learning how the node and blockchain
#  works, and how to interact with everything.

$CLI="jcli.exe"
$NODE="jormungandr.exe"

$WORKDIR=(get-location).path
$MYCLI=$WORKDIR + "/"+ $CLI
$MYNODE=$WORKDIR + "/"+ $NODE

write-host "WorkDir: $WORKDIR"
write-host "PS Version: " + $PSVersionTable.PSVersion

$REST_HOST="127.0.0.1"
$REST_PORT=8443
$REST_DEST=$REST_HOST+":"+$REST_PORT
$REST_URL="http://"+$REST_DEST+"/api"

$FEE_CONSTANT=10
$FEE_CERTIFICATE=0
$FEE_COEFFICIENT=0

$SLOT_DURATION=10
$SLOT_PER_EPOCH=5000

$FAUCET_AMOUNT=1000000000000
$FIXED_AMOUNT=1000000000000
$ADDRTYPE="--testing"

$CONSENSUS="genesis_praos"
$DATA_PATH="storage"
$SECRET_PATH="secret"
$CONFIG_PATH="config"
$ADD_STARTUP_SCRIPT=0

$STORAGE_PATH = $WORKDIR -replace "\\", "/"
$STORAGE_PATH = $STORAGE_PATH + "/" + $DATA_PATH + "/"

$BLOCK0_DATE=[int][double]::Parse((Get-Date(Get-Date).ToUniversalTime() -UFormat %s))


###  GET OR UPDATE THE BINARIES

[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$latestReleaseJson = curl https://api.github.com/repos/input-output-hk/jormungandr/releases/latest | ConvertFrom-Json
$latestReleaseTag = $latestReleaseJson.tag_name
$latestReleasePublishDate = $latestReleaseJson.published_at

if(![System.IO.File]::Exists($WORKDIR+"\"+$CLI)){
	# if there is no jcli.exe we download the latest release for Windows
	write-host "`n`nThere is no jcli.exe`nShould I download the latest release $latestReleaseTag ?" -ForegroundColor Yellow
	$Readhost = read-host "( Y / n )"
    Switch ($ReadHost) 
    { 
       Y {Write-host "Yes, download"; $DoUpdate=$true} 
       N {Write-Host "No, keep existing executables"; $DoUpdate=$false} 
       Default {Write-Host "Default, download"; $DoUpdate=$true} 
    }
	if ($DoUpdate) {
		$assetsJson = curl $latestReleaseJson.assets_url | ConvertFrom-Json
		foreach ($asset in $assetsJson.browser_download_url) {
			# looking for the "pc-windows-msvc" asset
			if ($asset -Match 'pc-windows-msvc') {
				write-host "Downloading $asset ..."
				curl $asset -outfile $WORKDIR\latestAsset.zip
				Add-Type -AssemblyName System.IO.Compression.FileSystem
				[System.IO.Compression.ZipFile]::ExtractToDirectory($WORKDIR+"\latestAsset.zip", $WORKDIR)
				Remove-Item $WORKDIR\latestAsset.zip
			}
		}
		$MYCLIVER= & $MYCLI --version
	} else {
		write-host "No executables no fun." -ForegroundColor RED
		exit
	}
	
} else {
	# if there is already a jcli.exe compare it's version with the latest available release
	$MYCLIVER= & $MYCLI --version
	$MYCLIVER = $MYCLIVER.replace("jcli ", "v") #fix version output to release tag format
	if ($MYCLIVER -ne $latestReleaseTag) {
		write-host "`n`There is  JCLI $MYCLIVER in use.`nShould I upgrade to $latestReleaseTag ?" -ForegroundColor Yellow
		$Readhost = read-host "( Y / n )"
		Switch ($ReadHost) 
		{ 
		   Y {Write-host "Yes, update"; $DoUpdate=$true} 
		   N {Write-Host "No, do nothing"; $DoUpdate=$false} 
		   Default {Write-Host "Default, update"; $DoUpdate=$true} 
		}
		if ($DoUpdate) {
			$assetsJson = curl $latestReleaseJson.assets_url | ConvertFrom-Json
			foreach ($asset in $assetsJson.browser_download_url) {
				# looking for the "pc-windows-msvc" asset
				if ($asset -Match 'pc-windows-msvc') {
					write-host "Downloading $asset ..."
					curl $asset -outfile $WORKDIR\latestAsset.zip
					Remove-Item $MYCLI
					Remove-Item $MYNODE
					Add-Type -AssemblyName System.IO.Compression.FileSystem
					[System.IO.Compression.ZipFile]::ExtractToDirectory($WORKDIR+"\latestAsset.zip", $WORKDIR)
					Remove-Item $WORKDIR\latestAsset.zip
				}
			}
		} 
		$MYCLIVER= & $MYCLI --version
		
	}
}

### MAKE EVERYTHING

if([System.IO.File]::Exists($MYCLI)){ 
	
	$MYCLIVER= & $MYCLI --version
	write-host "Using $MYCLIVER"

	# create or clean the CONFIG folder
	if(![System.IO.Directory]::Exists($WORKDIR+"\"+$CONFIG_PATH)){ 
		[System.IO.Directory]::CreateDirectory($WORKDIR+"\"+$CONFIG_PATH)
	} else {
		Write-host "Found an existing CONFIG folder ($CONFIG_PATH) Remove it? (Default is Yes)" -ForegroundColor Yellow 
    $Readhost = Read-Host " ( Y / n ) " 
    Switch ($ReadHost) 
     { 
       Y {Write-host "Yes, Remove former configuration"; $RemoveConfig=$false} 
       N {Write-Host "No, keep existing configuration"; $RemoveConfig=$true} 
       Default {Write-Host "Default, Remove former configuration"; $RemoveConfig=$false} 
     }
		if(!$RemoveConfig) {
			Get-ChildItem $WORKDIR"\"$CONFIG_PATH -Recurse | Remove-Item -Force
		}
	}
	# create or clean the DATA folder
	if(![System.IO.Directory]::Exists($WORKDIR+"\"+$DATA_PATH)){ 
		[System.IO.Directory]::CreateDirectory($WORKDIR+"\"+$DATA_PATH)
	} else {
		Write-host "Found an existing DATA folder ($DATA_PATH) Remove it? (Default is Yes)" -ForegroundColor Yellow 
    $Readhost = Read-Host " ( Y / n ) " 
    Switch ($ReadHost) 
     { 
       Y {Write-host "Yes, remove former database"; $RemoveData=$false} 
       N {Write-Host "No, keep existing database"; $RemoveData=$true} 
       Default {Write-Host "Default, remove former database"; $RemoveData=$false} 
     }
		if($RemoveData) {
			Get-ChildItem $WORKDIR"\"$DATA_PATH -Recurse | Remove-Item -Force
		}
	}
	# create or clean an existing SECRET folder
	if(![System.IO.Directory]::Exists($WORKDIR+"\"+$SECRET_PATH)){ 
		[System.IO.Directory]::CreateDirectory($WORKDIR+"\"+$SECRET_PATH)
	} else {
		Write-host "Found an existing SECRET folder ($SECRET_PATH) Remove it? (Default is Yes)" -ForegroundColor Yellow 
    $Readhost = Read-Host " ( y / n ) " 
    Switch ($ReadHost) 
     { 
       Y {Write-host "Yes, Remove former secrets"; $RemoveSecrets=$false} 
       N {Write-Host "No, keep existing secrets"; $RemoveSecrets=$true} 
       Default {Write-Host "Default, Remove former secrets"; $RemoveSecrets=$false} 
     }
		if(!$RemoveSecrets) {
			Get-ChildItem $WORKDIR"\"$SECRET_PATH -Recurse | Remove-Item -Force
		}
	}

	# faucet
	$FAUCET_SK = & $MYCLI key generate --type=Ed25519Extended
	write-host "FAUCET_SK ($FAUCET_SK)" -ForegroundColor DarkGreen
	$FAUCET_PK = echo $FAUCET_SK | & $MYCLI key to-public
	write-host "FAUCET_PK ($FAUCET_PK)" -ForegroundColor DarkGreen
	$FAUCET_ADDR= & $MYCLI address account ${ADDRTYPE} ${FAUCET_PK}
	write-host "FAUCET_ADDR ($FAUCET_ADDR)" -ForegroundColor DarkGreen
	write-host "Faucet keys: done" -ForegroundColor DarkGreen

	# fixed account
	$FIXED_SK = & $MYCLI key generate --type=Ed25519Extended
	$FIXED_PK = echo $FIXED_SK | & $MYCLI key to-public
	$FIXED_ADDR= & $MYCLI address account ${ADDRTYPE} ${FIXED_PK}
	write-host "Fixed keys: done" -ForegroundColor DarkGreen
	
	# leader
	$LEADER_SK = & $MYCLI key generate --type=Ed25519
	write-host "LEADER_SK" -ForegroundColor DarkGreen
	$LEADER_PK = echo $LEADER_SK | & $MYCLI key to-public
	write-host "LEADER_PK ($LEADER_PK)" -ForegroundColor DarkGreen
	write-host "leader keys: done" -ForegroundColor DarkGreen 

	# stake pool
	$POOL_VRF_SK = & $MYCLI key generate --type=Curve25519_2HashDH
	write-host "POOL_VRF_SK" -ForegroundColor DarkGreen
	$POOL_KES_SK = & $MYCLI key generate --type=SumEd25519_12
	write-host "POOL_KES_SK" -ForegroundColor DarkGreen
	$POOL_VRF_PK = echo $POOL_VRF_SK | & $MYCLI key to-public
	write-host "POOL_VRF_PK ($POOL_VRF_PK)" -ForegroundColor DarkGreen
	$POOL_KES_PK = echo $POOL_KES_SK | & $MYCLI key to-public
	write-host "POOL_KES_PK ($POOL_KES_PK)" -ForegroundColor DarkGreen
	# note we use the faucet as the owner to this pool
	$STAKE_KEY=$FAUCET_SK
	$STAKE_KEY_PUB=$FAUCET_PK
	echo $LEADER_SK | Out-File $WORKDIR"\"$SECRET_PATH\leader.sk -Encoding Oem
	echo $STAKE_KEY | Out-File $WORKDIR"\"$SECRET_PATH\stake_key.sk -Encoding Oem
	echo $FIXED_SK | Out-File $WORKDIR"\"$SECRET_PATH\fixed_key.sk -Encoding Oem
	echo $POOL_VRF_SK | Out-File $WORKDIR"\"$SECRET_PATH\stake_pool.vrf.sk -Encoding Oem
	echo $POOL_KES_SK | Out-File $WORKDIR"\"$SECRET_PATH\stake_pool.kes.sk -Encoding Oem
	write-host "stake-pool vrf and kes keys: done" -ForegroundColor DarkGreen

	$STAKEPOOLCERT = & $MYCLI certificate new stake-pool-registration --kes-key $POOL_KES_PK --vrf-key $POOL_VRF_PK --serial 1010101010 --management-threshold 1 --start-validity 0 --owner $LEADER_PK
	echo $STAKEPOOLCERT | Out-File $WORKDIR"\"$SECRET_PATH\stake_pool.cert -Encoding Oem
	$STAKEPOOLCERTSIGN = echo $STAKEPOOLCERT | & $MYCLI certificate sign -k $WORKDIR"\"$SECRET_PATH\leader.sk
	echo $STAKEPOOLCERTSIGN | Out-File $WORKDIR"\"$SECRET_PATH\stake_pool.signcert -Encoding Oem
	$STAKE_POOL_ID = echo $STAKEPOOLCERTSIGN | & $MYCLI certificate get-stake-pool-id
	write-host "stake-pool-registration certificate: done" -ForegroundColor DarkGreen

	$STAKEDELEGATION1 = & $MYCLI certificate new stake-delegation $STAKE_POOL_ID $FAUCET_PK 
	echo $STAKEDELEGATION1 | Out-File $WORKDIR"\"$SECRET_PATH\stake_delegation1.cert -Encoding Oem
	$STAKEDELEGATIONSIGN1 = $STAKEDELEGATION1 | & $MYCLI certificate sign -k $WORKDIR"\"$SECRET_PATH\stake_key.sk 
	echo $STAKEDELEGATIONSIGN1 | Out-File $WORKDIR"\"$SECRET_PATH\stake_delegation1.signcert -Encoding Oem
	write-host "stake-pool-delegation certificate 1: done" -ForegroundColor DarkGreen

	$STAKEDELEGATION2 = & $MYCLI certificate new stake-delegation $STAKE_POOL_ID $FIXED_PK
	echo $STAKEDELEGATION2 | Out-File $WORKDIR"\"$SECRET_PATH\stake_delegation2.cert -Encoding Oem
	$STAKEDELEGATIONSIGN2 = $STAKEDELEGATION2 | & $MYCLI certificate sign -k $WORKDIR"\"$SECRET_PATH\fixed_key.sk 
	echo $STAKEDELEGATIONSIGN2 | Out-File $WORKDIR"\"$SECRET_PATH\stake_delegation2.signcert -Encoding Oem
	write-host "stake-pool-delegation certificate 2: done" -ForegroundColor DarkGreen


	if(!$RemoveConfig) {
"blockchain_configuration:
  block0_date: $BLOCK0_DATE
  discrimination: test
  slots_per_epoch: $SLOT_PER_EPOCH
  slot_duration: $SLOT_DURATION
  epoch_stability_depth: 10
  consensus_genesis_praos_active_slot_coeff: 0.1
  consensus_leader_ids:
    - $LEADER_PK
  linear_fees:
    constant: $FEE_CONSTANT
    coefficient: $FEE_COEFFICIENT
    certificate: $FEE_CERTIFICATE
  block0_consensus: $CONSENSUS
  bft_slots_ratio: 0
  kes_update_speed: 43200 # 12hours
initial:
  - fund:
      - address: $FAUCET_ADDR
        value: $FAUCET_AMOUNT
      - address: $FIXED_ADDR
        value: $FIXED_AMOUNT
  - cert: $STAKEPOOLCERTSIGN
  - cert: $STAKEDELEGATIONSIGN1
  - cert: $STAKEDELEGATIONSIGN2" | Out-File $WORKDIR"\"$CONFIG_PATH\genesis.yaml -Encoding Oem
		write-host "genesis file generated: done" -ForegroundColor DarkGreen

"storage: ""$STORAGE_PATH""
log:
  level: debug
  format: plain
  output: stderr
rest:
  listen: ""127.0.0.1:48443""
p2p:
  trusted_peers: []
  public_address: ""/ip4/127.0.0.1/tcp/8299""
  topics_of_interest:
    messages: low
    blocks: normal" | Out-File $WORKDIR"\"$CONFIG_PATH\config.yaml -Encoding Oem
		write-host "configuration file: done" -ForegroundColor DarkGreen
	}	

	if(!$KeepSecrets) {
"genesis:
  sig_key: $POOL_KES_SK
  vrf_key: $POOL_VRF_SK
  node_id: $STAKE_POOL_ID" | Out-File $WORKDIR"\"$SECRET_PATH\poolsecret1.yaml -Encoding Oem
		write-host "poolsecret file: done" -ForegroundColor DarkGreen

		& $MYCLI genesis encode --input $WORKDIR"\"$CONFIG_PATH\genesis.yaml --output $WORKDIR"\"$CONFIG_PATH\block-0.bin
		write-host "genesis block binary encoded: done" -ForegroundColor DarkGreen
	}

    $Readhost = Read-Host "Do you want to start it now?  ( y / N ) " 
    Switch ($ReadHost) 
    { 
		Y { 
			& $MYNODE --genesis-block $CONFIG_PATH\block-0.bin --config $CONFIG_PATH\config.yaml --secret $SECRET_PATH\poolsecret1.yaml
		} 
		Default { 	
			write-host "OK. To manually start the node:"  -ForegroundColor GREEN
			write-host "./$NODE --genesis-block $CONFIG_PATH\block-0.bin --config $CONFIG_PATH\config.yaml --secret $SECRET_PATH\poolsecret1.yaml" -ForegroundColor GREEN
		} 
     }

} else { 
	write-host "ERROR: $MYCLI not found" -ForegroundColor RED
}
