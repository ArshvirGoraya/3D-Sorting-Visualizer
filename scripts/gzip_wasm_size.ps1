# This script is to check the size of the .wasm file (produced from Trunk) when it is compressed. 
# Rationale: Since HTTP protocols compress files before sending them, the .wasm file should be compressed to see what its final size will be when served to the user.

$wasm_dir = "./docs/"
$wasm_suffix = ".wasm"
$gz_suffix = ".gz"

$wasm_file_path = Get-ChildItem -Path $wasm_dir -File | Where-Object { $_.Name.EndsWith($wasm_suffix)}
$gz_file_path = Get-ChildItem -Path $wasm_dir -File | Where-Object { $_.Name.EndsWith($gz_suffix)}

$wasm_exists = -not([string]::IsNullOrEmpty($wasm_file_path))
$gz_exists = -not([string]::IsNullOrEmpty($gz_file_path))


function CompressWasm-PrintSize-DecompressGz-DeleteGz{
  Compress-File $wasm_file_path
  $gz_file_path = $wasm_file_path.FullName+$gz_suffix
  $bytes = Get-File-Size $gz_file_path
  # Write-Host "bytes: $bytes"
  Decompress-File $gz_file_path
  # Write-Host "removing: $gz_file_path"
  rm $gz_file_path
  Print-File-Size $bytes
}

function Compress-File{
  param (
    [string]$FilePath
  )
  # Write-Host "Gzipping: $FilePath"

  # -f overwrites any .gz that may already exist.
  # & gzip -9 -f $FilePath
  Start-Process -FilePath "gzip.exe" -ArgumentList "-9 -f $FilePath" -Wait -NoNewWindow
}

function Decompress-File{
  param (
    [string]$FilePath
  )
  # Write-Host "decompressing file"
  # -y overwrites any .wasm that may already exist.
  # > nul sends all outputs to nul window so dont have to see it (except errors)
  & 7z e -y -o"$wasm_dir" $FilePath > nul
  # Start-Process -FilePath "7z.exe" -ArgumentList "x -y $FilePath -o $wasm_dir" -Wait -NoNewWindow
}

function Get-File-Size{
  param (
    [string]$FilePath
  )
  # Using wc to get size... Powershell probably has an easier way of doing this, but just using gnu tools instead :)
  # Write-Host "calling wc: $FilePath"
  Start-Sleep -Milliseconds 1000
  $wc_output = & wc --bytes $FilePath
  # $wc_output = Start-Process -FilePath "wc.exe" -ArgumentList "--bytes $FilePath" -Wait

  $bytes = $wc_output.Split(" ")[0] # returns bytes + " " + name of file.... just need the bytes.
  return $bytes
}

function Print-File-Size{
  param (
    [int]$bytes
  )
  $bytes_kb = $bytes / 1024
  $bytes_mb = $bytes_kb / 1000
  # up to 2 decimal places
  
  $bytes_kb = "{0:N2}" -f $bytes_kb
  $bytes_mb = "{0:N2}" -f $bytes_mb

  Write-Host "wasm.gz: $bytes_mb(mb), $bytes_kb(kb)"

  if ($bytes_mb -gt 3.0){
    Write-Warning "wasm is a little large..."
  }else{
    Write-Host "wasm is OK size" -ForegroundColor Green
  }
}

# main

if ($wasm_exists -and $gz_exists){
  # Write-Warning ".wasm and .gz exist! re-compressing .wasm, getting .gz size and deleting .gz"
  CompressWasm-PrintSize-DecompressGz-DeleteGz
}
elseif ($gz_exists){
  # Write-Warning "only .gz exists! getting gz size, decompressing .wasm and deleting .gz"
  $bytes = Get-File-Size $gz_file_path
  Decompress-File $gz_file_path
  rm $gz_file_path
  Print-File-Size $bytes
}
elseif ($wasm_exists){
  # Write-Host ".wasm exists! compressing .wasm, getting size and deleting produced .gz"
  CompressWasm-PrintSize-DecompressGz-DeleteGz
}
else{
  Write-Error "no .wasm or .gz found!"
  exit 1
}

