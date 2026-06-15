import time

from proxmoxer import ProxmoxAPI  # ty:ignore[unresolved-import]

pve = ProxmoxAPI(
    "localhost", port=8006, user="root@pam", password="root", verify_ssl=False
)

node = pve.nodes("pve")

print("=== Creating fake Proxmox environment ===")

# Check available storages
print("\n[Check] Available storages:")
try:
    storages = node.storage.get()
    for s in storages:
        print(f"  - {s.get('storage', 'unknown')} (type: {s.get('type', 'unknown')})")
except Exception as e:
    print(f"  Could not list storages: {e}")

# Check available LXC templates
print("\n[Check] Available LXC templates:")
try:
    templates = node.storage("local").content.get(content="vztmpl")
    if templates:
        for t in templates:
            print(f"  - {t.get('volid', 'unknown')}")
    else:
        print("  No LXC templates found (LXC creation will be skipped)")
        templates = []
except Exception as e:
    print(f"  Could not list templates: {e}")
    templates = []

has_lxc_templates = len(templates) > 0

# ============================================================================
# 1. VMs (12 VMs with varied configurations)
# ============================================================================
print("\n[1/5] Creating VMs...")

created_vms = []
vms = [
    # Production VMs
    {
        "vmid": 100,
        "name": "web-01",
        "cores": 2,
        "memory": 2048,
        "disk": "8",
        "tags": "production,web",
        "desc": "Web server frontend",
    },
    {
        "vmid": 101,
        "name": "web-02",
        "cores": 2,
        "memory": 2048,
        "disk": "8",
        "tags": "production,web",
        "desc": "Web server frontend",
    },
    {
        "vmid": 102,
        "name": "db-01",
        "cores": 4,
        "memory": 8192,
        "disk": "32",
        "tags": "production,database",
        "desc": "PostgreSQL primary",
    },
    {
        "vmid": 103,
        "name": "db-02",
        "cores": 4,
        "memory": 8192,
        "disk": "32",
        "tags": "production,database",
        "desc": "PostgreSQL replica",
    },
    {
        "vmid": 104,
        "name": "app-01",
        "cores": 2,
        "memory": 4096,
        "disk": "16",
        "tags": "production,app",
        "desc": "Application server",
    },
    {
        "vmid": 105,
        "name": "app-02",
        "cores": 2,
        "memory": 4096,
        "disk": "16",
        "tags": "production,app",
        "desc": "Application server",
    },
    # Staging VMs
    {
        "vmid": 106,
        "name": "staging-web",
        "cores": 2,
        "memory": 2048,
        "disk": "8",
        "tags": "staging,web",
        "desc": "Staging web server",
    },
    {
        "vmid": 107,
        "name": "staging-db",
        "cores": 2,
        "memory": 4096,
        "disk": "16",
        "tags": "staging,database",
        "desc": "Staging database",
    },
    {
        "vmid": 108,
        "name": "staging-app",
        "cores": 2,
        "memory": 4096,
        "disk": "12",
        "tags": "staging,app",
        "desc": "Staging application",
    },
    # Development VMs
    {
        "vmid": 109,
        "name": "dev-workstation",
        "cores": 4,
        "memory": 8192,
        "disk": "40",
        "tags": "development,desktop",
        "desc": "Developer workstation",
    },
    {
        "vmid": 110,
        "name": "dev-test",
        "cores": 1,
        "memory": 1024,
        "disk": "4",
        "tags": "development,test",
        "desc": "CI test runner",
    },
    # Windows VM
    {
        "vmid": 111,
        "name": "win10",
        "cores": 4,
        "memory": 8192,
        "disk": "60",
        "tags": "desktop,windows",
        "desc": "Windows 10 desktop",
    },
]

for vm in vms:
    try:
        node.qemu.create(
            vmid=vm["vmid"],
            name=vm["name"],
            cores=vm["cores"],
            memory=vm["memory"],
            scsi0=f"local:{vm['disk']}",
            net0="virtio,bridge=vmbr0",
            ostype="l26",
            tags=vm.get("tags", ""),
            description=vm.get("desc", ""),
            boot="order=scsi0",
        )
        print(
            f"  Created VM {vm['vmid']}: {vm['name']} ({vm['cores']} cores, {vm['memory']}MB)"
        )
        created_vms.append(vm)
        time.sleep(0.5)
    except Exception as e:
        print(f"  Warning: VM {vm['vmid']} creation issue: {e}")

# ============================================================================
# 2. LXC Containers (6 containers) — skipped if no templates available
# ============================================================================
print("\n[2/5] Creating LXC containers...")

created_cts = []
containers = [
    {
        "vmid": 200,
        "hostname": "ct-proxy",
        "cores": 1,
        "memory": 512,
        "disk": "4",
        "tags": "production,proxy",
        "desc": "Nginx reverse proxy",
    },
    {
        "vmid": 201,
        "hostname": "ct-cache",
        "cores": 1,
        "memory": 1024,
        "disk": "8",
        "tags": "production,cache",
        "desc": "Redis cache server",
    },
    {
        "vmid": 202,
        "hostname": "ct-mq",
        "cores": 2,
        "memory": 2048,
        "disk": "8",
        "tags": "production,messaging",
        "desc": "RabbitMQ message queue",
    },
    {
        "vmid": 203,
        "hostname": "ct-monitor",
        "cores": 2,
        "memory": 4096,
        "disk": "16",
        "tags": "production,monitoring",
        "desc": "Prometheus + Grafana",
    },
    {
        "vmid": 204,
        "hostname": "ct-backup",
        "cores": 1,
        "memory": 1024,
        "disk": "32",
        "tags": "production,backup",
        "desc": "Backup server",
    },
    {
        "vmid": 205,
        "hostname": "ct-ansible",
        "cores": 2,
        "memory": 2048,
        "disk": "8",
        "tags": "development,ansible",
        "desc": "Ansible control node",
    },
]

if not has_lxc_templates:
    print("  Skipping LXC creation — no templates available")
else:
    for ct in containers:
        try:
            node.lxc.create(
                vmid=ct["vmid"],
                hostname=ct["hostname"],
                cores=ct["cores"],
                memory=ct["memory"],
                rootfs=f"local:{ct['disk']}",
                net0="name=eth0,bridge=vmbr0,ip=dhcp",
                ostemplate="local:vztmpl/ubuntu-22.04-standard_22.04-1_amd64.tar.gz",
                tags=ct.get("tags", ""),
                description=ct.get("desc", ""),
            )
            print(
                f"  Created CT {ct['vmid']}: {ct['hostname']} ({ct['cores']} cores, {ct['memory']}MB)"
            )
            created_cts.append(ct)
            time.sleep(0.5)
        except Exception as e:
            print(f"  Warning: CT {ct['vmid']} creation issue: {e}")

# ============================================================================
# 3. Pools
# ============================================================================
print("\n[3/5] Creating pools...")

pools = [
    {"poolid": "production", "comment": "Production environment - customer facing"},
    {"poolid": "staging", "comment": "Staging environment - pre-release validation"},
    {"poolid": "development", "comment": "Development environment - internal testing"},
]

for pool in pools:
    try:
        pve.pools.create(poolid=pool["poolid"], comment=pool["comment"])
        print(f"  Created pool: {pool['poolid']}")
    except Exception as e:
        print(f"  Warning: Pool {pool['poolid']} issue: {e}")

# ============================================================================
# 4. Assign VMs to pools
# ============================================================================
print("\n[4/5] Assigning resources to pools...")

pool_assignments = {
    "production": [100, 101, 102, 103, 104, 105] + [ct["vmid"] for ct in created_cts if ct["vmid"] <= 203],
    "staging": [106, 107, 108] + [ct["vmid"] for ct in created_cts if ct["vmid"] == 204],
    "development": [109, 110, 111] + [ct["vmid"] for ct in created_cts if ct["vmid"] == 205],
}

for poolid, vmids in pool_assignments.items():
    for vmid in vmids:
        try:
            pve.pools(poolid).put(vms=vmid)
            print(f"  Assigned {vmid} -> {poolid}")
        except Exception as e:
            print(f"  Warning: Assign {vmid} to {poolid} issue: {e}")

# ============================================================================
# 5. Snapshots
# ============================================================================
print("\n[5/5] Creating snapshots...")

snapshots = [
    {"vmid": 100, "snapname": "pre-deploy", "desc": "Before v2.0 deployment"},
    {"vmid": 100, "snapname": "post-deploy", "desc": "After v2.0 deployment"},
    {"vmid": 102, "snapname": "baseline", "desc": "Clean database state"},
    {"vmid": 102, "snapname": "pre-migration", "desc": "Before schema migration"},
    {"vmid": 104, "snapname": "stable", "desc": "Known good version"},
    {"vmid": 106, "snapname": "test-start", "desc": "Initial staging test"},
    {"vmid": 109, "snapname": "dev-base", "desc": "Developer workstation base"},
]

for snap in snapshots:
    try:
        node.qemu(snap["vmid"]).snapshot.create(
            snapname=snap["snapname"],
            description=snap["desc"],
        )
        print(f"  Created snapshot '{snap['snapname']}' on VM {snap['vmid']}")
    except Exception as e:
        print(f"  Warning: Snapshot {snap['snapname']} on {snap['vmid']} issue: {e}")

# ============================================================================
# 6. Start some VMs to get realistic status
# ============================================================================
print("\n[Extra] Starting some VMs for realistic status...")

running_vms = [100, 101, 102, 104, 106, 109, 111]
for vmid in running_vms:
    try:
        node.qemu(vmid).status.start.post()
        print(f"  Started VM {vmid}")
        time.sleep(1)
    except Exception as e:
        print(f"  Warning: Start VM {vmid} issue: {e}")

# ============================================================================
# 7. Summary
# ============================================================================
print("\n" + "=" * 60)
print("Fake Proxmox environment created successfully!")
print("=" * 60)
print(f"  VMs:        {len(created_vms)} / {len(vms)} (qemu)")
print(f"  Containers: {len(created_cts)} / {len(containers)} (lxc)")
print(f"  Pools:      {len(pools)}")
print(f"  Snapshots:  {len(snapshots)}")
print(f"  Running:    {len(running_vms)} VMs started")
print("\nYou can now connect with p9s:")
print("  p9s --endpoint https://127.0.0.1:8006 --insecure")
print("=" * 60)
