#define AppName "Atlas"
#define AppVersion GetEnv("ATLAS_VERSION")
#define AppPublisher "Atlas Client"
#define AppExeName "atlas-client.exe"

[Setup]
AppId={{8B420F3A-D707-4D33-9D8A-51A18E75AC11}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
DefaultDirName={localappdata}\Programs\Atlas
DefaultGroupName=Atlas
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir=dist\windows
OutputBaseFilename=atlas-client-windows-x86_64-setup
ArchitecturesInstallIn64BitMode=x64
ArchitecturesAllowed=x64
WizardStyle=modern dark slate includetitlebar
WizardSizePercent=100
SetupLogging=yes
UninstallDisplayIcon={app}\atlas-client.exe
CloseApplications=yes
RestartApplications=no
SolidCompression=yes
Compression=lzma2/ultra64

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked

[Files]
Source: "dist\windows\atlas-client.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "dist\windows\uninstall.ps1"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Atlas"; Filename: "{app}\atlas-client.exe"; WorkingDir: "{app}"
Name: "{autodesktop}\Atlas"; Filename: "{app}\atlas-client.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\atlas-client.exe"; Description: "Launch Atlas"; Flags: postinstall nowait

[Code]
var
  StageLabel: TNewStaticText;
  StageDetail: TNewStaticText;
  ProgressGauge: TNewProgressBar;

procedure InitializeWizard;
begin
  WizardForm.Caption := 'Atlas Setup';
  WizardForm.StatusLabel.Visible := False;
  WizardForm.FilenameLabel.Visible := False;
  WizardForm.WizardSmallBitmapImage.Visible := False;
  WizardForm.WizardBitmapImage.Visible := False;
  WizardForm.WizardBitmapImage2.Visible := False;
  StageLabel := TNewStaticText.Create(WizardForm);
  StageLabel.Parent := WizardForm.InstallingPage;
  StageLabel.Left := WizardForm.StatusLabel.Left;
  StageLabel.Top := WizardForm.StatusLabel.Top + 8;
  StageLabel.Width := WizardForm.StatusLabel.Width;
  StageLabel.Height := 28;
  StageLabel.Font.Size := 15;
  StageLabel.Font.Style := [fsBold];
  StageLabel.Font.Color := $9CAB86;
  StageLabel.Caption := 'Preparing your game space';
  StageDetail := TNewStaticText.Create(WizardForm);
  StageDetail.Parent := WizardForm.InstallingPage;
  StageDetail.Left := StageLabel.Left;
  StageDetail.Top := StageLabel.Top + 32;
  StageDetail.Width := StageLabel.Width;
  StageDetail.Height := 22;
  StageDetail.Font.Color := $B0B0B0;
  StageDetail.Caption := 'Unpacking Atlas and creating your shortcuts';
  ProgressGauge := TNewProgressBar.Create(WizardForm);
  ProgressGauge.Parent := WizardForm.InstallingPage;
  ProgressGauge.Left := WizardForm.ProgressGauge.Left;
  ProgressGauge.Top := StageDetail.Top + 32;
  ProgressGauge.Width := WizardForm.ProgressGauge.Width;
  ProgressGauge.Height := 12;
  ProgressGauge.Min := 0;
  ProgressGauge.Max := 100;
  ProgressGauge.Position := 0;
  WizardForm.ProgressGauge.Visible := False;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssInstall then begin
    StageLabel.Caption := 'Making Atlas yours';
    StageDetail.Caption := 'Placing files, setting up your Start Menu, and getting things ready';
  end;
  if CurStep = ssPostInstall then begin
    StageLabel.Caption := 'Ready for takeoff';
    StageDetail.Caption := 'Atlas is installed. Your next world is one click away.';
    ProgressGauge.Position := 100;
  end;
end;

procedure CurInstallProgressChanged(CurProgress, MaxProgress: Integer);
begin
  if MaxProgress > 0 then begin
    ProgressGauge.Position := (CurProgress * 100) div MaxProgress;
    if CurProgress < MaxProgress div 5 then begin
      StageLabel.Caption := 'Unpacking Atlas';
      StageDetail.Caption := 'Putting the launcher files in place';
    end else if CurProgress < (MaxProgress * 3) div 4 then begin
      StageLabel.Caption := 'Setting up your space';
      StageDetail.Caption := 'Preparing Atlas for this device';
    end else if CurProgress < (MaxProgress * 19) div 20 then begin
      StageLabel.Caption := 'Creating your shortcuts';
      StageDetail.Caption := 'Adding Atlas to your Start Menu';
    end else begin
      StageLabel.Caption := 'Finishing up';
      StageDetail.Caption := 'Almost ready for your next world';
    end;
  end;
end;
