import time

from proxmoxer import ProxmoxAPI  # ty:ignore[unresolved-import]


def create_if_missing(
    resource_id: str,
    existing: set,
    create_fn,
    created: list | None,
    resource_type: str,
) -> None:
    if resource_id in existing:
        print(f"  Skipping {resource_type} {resource_id}: already exists")
        return
    try:
        create_fn()
        print(f"  Created {resource_type}: {resource_id}")
        if created is not None:
            created.append(resource_id)
        existing.add(resource_id)
    except Exception as e:
        print(f"  Warning: {resource_type} {resource_id} issue: {e}")


def connect():
    pve = ProxmoxAPI(
        "localhost", port=8006, user="root@pam", password="root", verify_ssl=False
    )
    return pve, pve.nodes("pve")


def check_storages(node):
    print("\n[Check] Available storages:")
    try:
        storages = node.storage.get()
        for s in storages:
            print(f"  - {s.get('storage', 'unknown')} (type: {s.get('type', 'unknown')})")
    except Exception as e:
        print(f"  Could not list storages: {e}")


def check_templates(node):
    print("\n[Check] Available LXC templates:")
    try:
        templates = node.storage("local").content.get(content="vztmpl")
        if templates:
            for t in templates:
                print(f"  - {t.get('volid', 'unknown')}")
        else:
            print("  No LXC templates found (LXC creation will be skipped)")
    except Exception as e:
        print(f"  Could not list templates: {e}")
        templates = []
    return templates


def collect_existing_resources(pve, node):
    print("\n[Check] Collecting existing resources...")
    existing = {
        "vmids": set(),
        "ctids": set(),
        "pools": set(),
        "sdn_zones": set(),
        "sdn_vnets": set(),
        "replication": set(),
        "ha": set(),
        "backups": set(),
    }
    try:
        existing["vmids"] = {vm.get("vmid") for vm in node.qemu.get()}
    except Exception as e:
        print(f"  Could not list VMs: {e}")
    try:
        existing["ctids"] = {ct.get("vmid") for ct in node.lxc.get()}
    except Exception as e:
        print(f"  Could not list containers: {e}")
    try:
        existing["pools"] = {pool.get("poolid") for pool in pve.pools.get()}
    except Exception as e:
        print(f"  Could not list pools: {e}")
    try:
        existing["sdn_zones"] = {
            zone.get("zone") for zone in pve.cluster.sdn.zones.get()
        }
    except Exception as e:
        print(f"  Could not list SDN zones: {e}")
    try:
        existing["sdn_vnets"] = {
            vnet.get("vnet") for vnet in pve.cluster.sdn.vnets.get()
        }
    except Exception as e:
        print(f"  Could not list SDN VNets: {e}")
    try:
        existing["replication"] = {
            job.get("id") for job in pve.cluster.replication.get()
        }
    except Exception as e:
        print(f"  Could not list replication jobs: {e}")
    try:
        existing["ha"] = {ha.get("sid") for ha in pve.cluster.ha.resources.get()}
    except Exception as e:
        print(f"  Could not list HA resources: {e}")
    try:
        existing["backups"] = {backup.get("id") for backup in pve.cluster.backup.get()}
    except Exception as e:
        print(f"  Could not list backup jobs: {e}")
    return existing


def create_vms(node, existing_vmids):
    print("\n[1/11] Creating VMs...")
    vms = [
        {"vmid": 100, "name": "web-01", "cores": 2, "memory": 2048, "disk": "8", "tags": "production,web", "desc": "Web server frontend"},
        {"vmid": 101, "name": "web-02", "cores": 2, "memory": 2048, "disk": "8", "tags": "production,web", "desc": "Web server frontend"},
        {"vmid": 102, "name": "db-01", "cores": 4, "memory": 8192, "disk": "32", "tags": "production,database", "desc": "PostgreSQL primary"},
        {"vmid": 103, "name": "db-02", "cores": 4, "memory": 8192, "disk": "32", "tags": "production,database", "desc": "PostgreSQL replica"},
        {"vmid": 104, "name": "app-01", "cores": 2, "memory": 4096, "disk": "16", "tags": "production,app", "desc": "Application server"},
        {"vmid": 105, "name": "app-02", "cores": 2, "memory": 4096, "disk": "16", "tags": "production,app", "desc": "Application server"},
        {"vmid": 106, "name": "staging-web", "cores": 2, "memory": 2048, "disk": "8", "tags": "staging,web", "desc": "Staging web server"},
        {"vmid": 107, "name": "staging-db", "cores": 2, "memory": 4096, "disk": "16", "tags": "staging,database", "desc": "Staging database"},
        {"vmid": 108, "name": "staging-app", "cores": 2, "memory": 4096, "disk": "12", "tags": "staging,app", "desc": "Staging application"},
        {"vmid": 109, "name": "dev-workstation", "cores": 4, "memory": 8192, "disk": "40", "tags": "development,desktop", "desc": "Developer workstation"},
        {"vmid": 110, "name": "dev-test", "cores": 1, "memory": 1024, "disk": "4", "tags": "development,test", "desc": "CI test runner"},
        {"vmid": 111, "name": "win10", "cores": 4, "memory": 8192, "disk": "60", "tags": "desktop,windows", "desc": "Windows 10 desktop"},
    ]
    created = []
    for vm in vms:
        if vm["vmid"] in existing_vmids:
            print(f"  Skipping VM {vm['vmid']}: already exists")
            continue
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
            print(f"  Created VM {vm['vmid']}: {vm['name']} ({vm['cores']} cores, {vm['memory']}MB)")
            created.append(vm)
            existing_vmids.add(vm["vmid"])
            time.sleep(0.5)
        except Exception as e:
            print(f"  Warning: VM {vm['vmid']} creation issue: {e}")
    return created, vms


def create_lxc(node, has_lxc_templates, existing_ctids):
    print("\n[2/11] Creating LXC containers...")
    containers = [
        {"vmid": 200, "hostname": "ct-proxy", "cores": 1, "memory": 512, "disk": "4", "tags": "production,proxy", "desc": "Nginx reverse proxy"},
        {"vmid": 201, "hostname": "ct-cache", "cores": 1, "memory": 1024, "disk": "8", "tags": "production,cache", "desc": "Redis cache server"},
        {"vmid": 202, "hostname": "ct-mq", "cores": 2, "memory": 2048, "disk": "8", "tags": "production,messaging", "desc": "RabbitMQ message queue"},
        {"vmid": 203, "hostname": "ct-monitor", "cores": 2, "memory": 4096, "disk": "16", "tags": "production,monitoring", "desc": "Prometheus + Grafana"},
        {"vmid": 204, "hostname": "ct-backup", "cores": 1, "memory": 1024, "disk": "32", "tags": "production,backup", "desc": "Backup server"},
        {"vmid": 205, "hostname": "ct-ansible", "cores": 2, "memory": 2048, "disk": "8", "tags": "development,ansible", "desc": "Ansible control node"},
    ]
    created = []
    if not has_lxc_templates:
        print("  Skipping LXC creation — no templates available")
        return created, containers
    for ct in containers:
        if ct["vmid"] in existing_ctids:
            print(f"  Skipping CT {ct['vmid']}: already exists")
            continue
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
            print(f"  Created CT {ct['vmid']}: {ct['hostname']} ({ct['cores']} cores, {ct['memory']}MB)")
            created.append(ct)
            existing_ctids.add(ct["vmid"])
            time.sleep(0.5)
        except Exception as e:
            print(f"  Warning: CT {ct['vmid']} creation issue: {e}")
    return created, containers


def create_pools(pve, existing_pools):
    print("\n[3/11] Creating pools...")
    pools = [
        {"poolid": "production", "comment": "Production environment - customer facing"},
        {"poolid": "staging", "comment": "Staging environment - pre-release validation"},
        {"poolid": "development", "comment": "Development environment - internal testing"},
    ]
    for pool in pools:
        create_if_missing(
            pool["poolid"],
            existing_pools,
            lambda p=pool: pve.pools.create(poolid=p["poolid"], comment=p["comment"]),
            None,
            "pool",
        )
    return pools


def assign_pools(pve, created_cts):
    print("\n[4/11] Assigning resources to pools...")
    pool_assignments = {
        "production": [100, 101, 102, 103, 104, 105]
        + [ct["vmid"] for ct in created_cts if ct["vmid"] <= 203],  # ty:ignore[unsupported-operator]
        "staging": [106, 107, 108]
        + [ct["vmid"] for ct in created_cts if ct["vmid"] == 204],
        "development": [109, 110, 111]
        + [ct["vmid"] for ct in created_cts if ct["vmid"] == 205],
    }
    for poolid, vmids in pool_assignments.items():
        try:
            members = pve.pools(poolid).get().get("members", [])
            assigned = {m.get("vmid") for m in members}
        except Exception:
            assigned = set()
        for vmid in vmids:
            if vmid in assigned:
                print(f"  Skipping {vmid} -> {poolid}: already a member")
                continue
            try:
                pve.pools(poolid).put(vms=vmid)
                print(f"  Assigned {vmid} -> {poolid}")
            except Exception as e:
                print(f"  Warning: Assign {vmid} to {poolid} issue: {e}")


def create_snapshots(node):
    print("\n[5/11] Creating snapshots...")
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
    return snapshots


def create_sdn(pve, existing):
    print("\n[6/11] Creating SDN zones and VNets...")
    created_sdn = []
    sdn_zones = [
        {"zone": "vlanz", "type": "vlan", "bridge": "vmbr0"},
    ]
    sdn_vnets = [
        {"vnet": "prodnet", "zone": "vlanz", "tag": 100},
        {"vnet": "devnet", "zone": "vlanz", "tag": 200},
    ]
    for zone in sdn_zones:
        create_if_missing(
            zone["zone"],
            existing["sdn_zones"],
            lambda z=zone: pve.cluster.sdn.zones.create(**z),
            created_sdn,
            "SDN zone",
        )
    for vnet in sdn_vnets:
        create_if_missing(
            vnet["vnet"],
            existing["sdn_vnets"],
            lambda v=vnet: pve.cluster.sdn.vnets.create(**v),
            created_sdn,
            "SDN VNet",
        )
    try:
        pve.cluster.sdn.put()
        print("  Applied SDN configuration")
    except Exception as e:
        print(f"  Warning: SDN apply issue: {e}")
    return created_sdn


def create_replication(pve, existing):
    print("\n[7/11] Creating replication jobs...")
    created_replication = []
    try:
        node_count = len(pve.nodes.get())
    except Exception:
        node_count = 1
    if node_count < 2:
        print("  Skipping replication — requires at least 2 cluster nodes")
        return created_replication
    replication_jobs = [
        {"id": "100-0", "type": "local", "target": "pve", "schedule": "*/15"},
        {"id": "102-0", "type": "local", "target": "pve", "schedule": "02:00"},
    ]
    for job in replication_jobs:
        create_if_missing(
            job["id"],
            existing["replication"],
            lambda j=job: pve.cluster.replication.create(**j),
            created_replication,
            "replication job",
        )
    return created_replication


def trigger_tasks(node):
    print("\n[8/11] Triggering cluster tasks...")
    created_tasks = []
    actions = [
        (103, "start", node.qemu(103).status.start.post),
        (105, "start", node.qemu(105).status.start.post),
        (107, "start", node.qemu(107).status.start.post),
    ]
    for vmid, action_name, action_fn in actions:
        try:
            task = action_fn()
            print(f"  Triggered {action_name} task on VM {vmid}: {task}")
            created_tasks.append(task)
        except Exception as e:
            print(f"  Warning: {action_name.capitalize()} task on VM {vmid} issue: {e}")
    return created_tasks


def create_ha_resources(pve, existing):
    print("\n[9/11] Creating HA resources...")
    created_ha = []
    ha_resources = [
        {"sid": "vm:100", "state": "started", "max_restart": 1, "max_relocate": 1},
        {"sid": "vm:102", "state": "started", "max_restart": 1, "max_relocate": 1},
        {"sid": "vm:104", "state": "stopped"},
    ]
    for ha in ha_resources:
        create_if_missing(
            ha["sid"],
            existing["ha"],
            lambda h=ha: pve.cluster.ha.resources.create(**h),
            created_ha,
            "HA resource",
        )
    return created_ha


def create_backups(pve, existing):
    print("\n[10/11] Creating backup jobs...")
    created_backups = []
    backups = [
        {"id": "backup-vm-100", "vmid": "100", "schedule": "02:00", "enabled": 1, "mode": "stop", "storage": "local"},
        {"id": "backup-vm-102", "vmid": "102", "schedule": "sun 03:00", "enabled": 1, "mode": "suspend", "storage": "local"},
    ]
    for backup in backups:
        create_if_missing(
            backup["id"],
            existing["backups"],
            lambda b=backup: pve.cluster.backup.create(**b),
            created_backups,
            "backup job",
        )
    return created_backups


def list_node_disks(node):
    print("\n[11/11] Listing node disks...")
    disks = []
    try:
        disks = node.disks.list.get()
        for disk in disks:
            print(f"  Disk: {disk.get('devpath', 'unknown')} ({disk.get('model', 'unknown')})")
    except Exception as e:
        print(f"  Warning: Could not list node disks: {e}")
    return disks


def start_vms(node):
    print("\n[Extra] Starting some VMs for realistic status...")
    running_vms = [100, 101, 102, 104, 106, 109, 111]
    for vmid in running_vms:
        try:
            node.qemu(vmid).status.start.post()
            print(f"  Started VM {vmid}")
            time.sleep(1)
        except Exception as e:
            print(f"  Warning: Start VM {vmid} issue: {e}")
    return running_vms


def print_summary(created_vms, vms, created_cts, containers, pools, snapshots, created_sdn, created_replication, created_tasks, created_ha, created_backups, disks, running_vms):
    print("\n" + "=" * 60)
    print("Fake Proxmox environment created successfully!")
    print("=" * 60)
    print(f"  VMs:          {len(created_vms)} / {len(vms)} (qemu)")
    print(f"  Containers:   {len(created_cts)} / {len(containers)} (lxc)")
    print(f"  Pools:        {len(pools)}")
    print(f"  Snapshots:    {len(snapshots)}")
    print(f"  SDN objects:  {len(created_sdn)}")
    print(f"  Replication:  {len(created_replication)}")
    print(f"  Tasks:        {len(created_tasks)}")
    print(f"  HA resources: {len(created_ha)}")
    print(f"  Backups:      {len(created_backups)}")
    print(f"  Disks:        {len(disks)}")
    print(f"  Running:      {len(running_vms)} VMs started")
    print("\nYou can now connect with p9s:")
    print("  p9s --endpoint https://127.0.0.1:8006 --insecure")
    print("=" * 60)


def main():
    print("=== Creating fake Proxmox environment ===")
    pve, node = connect()
    check_storages(node)
    templates = check_templates(node)
    has_lxc_templates = len(templates) > 0

    existing = collect_existing_resources(pve, node)

    created_vms, vms = create_vms(node, existing["vmids"])
    created_cts, containers = create_lxc(node, has_lxc_templates, existing["ctids"])
    pools = create_pools(pve, existing["pools"])
    assign_pools(pve, created_cts)
    snapshots = create_snapshots(node)
    created_sdn = create_sdn(pve, existing)
    created_replication = create_replication(pve, existing)
    created_tasks = trigger_tasks(node)
    created_ha = create_ha_resources(pve, existing)
    created_backups = create_backups(pve, existing)
    disks = list_node_disks(node)
    running_vms = start_vms(node)

    print_summary(
        created_vms,
        vms,
        created_cts,
        containers,
        pools,
        snapshots,
        created_sdn,
        created_replication,
        created_tasks,
        created_ha,
        created_backups,
        disks,
        running_vms,
    )


if __name__ == "__main__":
    main()
