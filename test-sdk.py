import requests
import copy
import time
import uuid
import json
import pprint
import hmac
import base64
import argparse

app_id = "a2faae91-d52f-497d-9029-d91be08c28c5"
app_config = {
        app_id : {
            "token"          : bytearray.fromhex('46732a28cd445366c6c8dcbd57500af4e69597c8ebe224634d6ccab812275c9c'),
            "secret_android" : bytearray.fromhex('1b66af517dd60807aeff8b4582d202ef500085bc0cec92bc3e67f0c58d6203b5'),
            "secret_ios"     : bytearray.fromhex('1b66af517dd60807aeff8b4582d202ef500085bc0cec92bc3e67f0c58d6203b5'),
            "secret_web"     : bytearray.fromhex('4c553960fdc2a82f90b84f6ef188e836818fcee2c43a6c32bd6c91f41772657f'),
            }
        }

token           = app_config[app_id]['token']
secret_android  = app_config[app_id]['secret_android']
secret_ios      = app_config[app_id]['secret_ios']
secret_web      = app_config[app_id]['secret_web']

path = '/xray/events/360dialog/sdk/v1'
URL = 'http://localhost:1337{}'.format(path)
v2_app_instance_id = "this-is-an-app-instance-id"
device_id = None #"hh30hrGVNrBTz/5bOerdBn5/Vz9zOx9G5hpavkuRWn9ECAWwsyubeA+SFDBdZ3AXeyFPF7JDGXy0ctAxtuAsxA=="
s = requests.Session()

base_data = {
    'environment': {
        'sdk_version': '1.0.1',
        'app_version': '1.0',
        'app_store_id': "com.me",
        'app_id': app_id,
        'app_instance_id': "1",
    },
    'device': {
        'platform' : 'android',
        'carrier_country': 'de',
        'ifa_tracking_enabled': True,
        'ifa': '594e7a0a-e7be-4d8b-a6de-51aafded6db7',
        'carrier_name': '1&1',
        'locale': 'fr',
        'time_zone': 'Europe/Berlin',
        'manufacturer': 'Google',
        'model': 'nexus 10',
        'os_version': '5',
        'os_name' : "iOS",
        "device_name": "Jan's iPhone",
        "notification_registered": True,
        "notification_types": 7,
        'network_connection_type': 'wifi'
    },
    'events': [
    ],
}

def make_event(event):
    cpy = copy.deepcopy(base_data)
    if type(event) == type([]):
        cpy['events'] = event
    else:
        cpy['events'].append(event)

    for e in cpy['events']:
        e['id'] = str(uuid.uuid4())
        if 'timestamp' not in e:
            e['timestamp'] = str(int(time.time()) * 1000)

    return json.dumps(cpy)

def post(payload, headers=None):
    data = make_event(payload).encode('utf-8')
    headers = headers or {}
    platform = base_data['device']['platform']
    if platform == 'android' and secret_android:
        headers['D360-Signature'] = base64.b64encode(hmac.new(
            secret_android, data, "SHA512").digest())
    elif platform == 'ios' and secret_ios:
        headers['D360-Signature'] = base64.b64encode(hmac.new(
            secret_ios, data, "SHA512").digest())
    elif platform == 'web' and secret_web:
        headers['D360-Signature'] = base64.b64encode(hmac.new(
            secret_web, data, "SHA512").digest())
    else:
       raise Exception("Device platform " + platform + " is unsupported")
     

    result = s.post(URL, data=data, headers=headers)
    res = handle_result(result)
    return res

def handle_result(response):
    if response.status_code not in (200, 201):
        raise Exception('invalid response code: 200 != {}\nbody: {}'.format(response.status_code, response.text))

    j = response.json()

    print(j)

    if not j:
        raise Exception('Empty json')

    status = j.get('events_status', [])
    if not status:
        raise Exception('empty response')

    e_resp = status[0]
    if e_resp['status'] != 'success':
        raise Exception('Event was not successfuly handled')

    return e_resp

def get_device_id():
    global token
    registration_data = {
        'name': 'd360_register',
        'properties': {
            'v2_app_instance_id': v2_app_instance_id
        }
    }
    res = post(registration_data)

    data = res['registration_data']
    token = data['api_token']
    return data['device_id']


default_header = {
    'X-Real-IP': '1.2.3.4'
}

def default_auth():
    if not device_id:
        return default_header
    result = default_header.copy()
    result.update({
        'D360-Device-Id': device_id,
        'D360-Api-Token': token
    })
    return result


def test_reusing_same_did():
    global token
    registration_data = {
        'name': 'd360_register',
        'properties': {
            'v2_app_instance_id': v2_app_instance_id
        }
    }
    auth = default_auth()
    del auth['D360-Api-Token']
    did = auth['D360-Device-Id']

    res = post(registration_data, auth)
    data = res['registration_data']
    if data['device_id'] != did:
        raise Exception("re-registering the same did should return the old did")
    token = data['api_token']

def send_token_update():
    push_token_update = {
        'properties': {
            'device_token': 'cf2d656e9d79274742912ef10e79653e151284807764220b3c75cfa24d118a0a'
        },
        'name': 'd360_push_token_update'
    }
    result = post(push_token_update, default_auth())

def send_app_open():
    push_token_update = {
        'properties': {
            'cold_start': True
        },
        'name': 'd360_app_open'
    }
    result = post(push_token_update, default_auth())

def send_app_close():
    push_token_update = {
        'properties': {
        },
        'name': 'd360_app_close'
    }
    result = post(push_token_update, default_auth())

def send_purchase(currency = 'EUR', price = 10):
    data = {
        'properties': {
            'currency': currency,
            'price': price,
        },
        'name': 'd360_app_shop_purchase',
    }
    result = post(data, default_auth())


def send_session_start():
    data = {
        'properties': {
        },
        'session_id': '{}'.format(uuid.uuid4()),
        'name': 'd360_session_start',
    }
    result = post(data, default_auth())

def send_empty():
    result = post([], default_auth())

def send_mdn_update():
    mdn_update = {
            'properties': {
                'mdn': '4915117534571'
            },
            'name': 'd360_app_mdn_update'
    }
    result = post(mdn_update, default_auth())

def send_batch():
    data = [
        { 'name': 'd1', 'timestamp': '8', },
        { 'name': 'd2', 'timestamp': '7', },
        { 'name': 'd3', 'timestamp': '6', },
        { 'name': 'd4', 'timestamp': '5', },
        { 'name': 'd5', 'timestamp': '4', },
        { 'name': 'd6', 'timestamp': '3', },
        { 'name': 'd7', 'timestamp': '2', },
        { 'name': 'd8', 'timestamp': '1', },
    ]
    result = post(data, default_auth())


def send_push_token_update():
    data = {
        'properties': {
            'endpoint' : "https://android.googleapis.com/gcm/send/fWBsd9rBW5s:APA91bEY7aIbvfebPrrl1nVwA19f7hADBxbPDSYwgdXh-ibcIX9cwlHqyNnNbZ-2pgWicJQLppyTxY-9RhNoSoKza8dzXdQgIZWxFnioMgVs6c-r43InTVB1NwfpDmJN7J26sLEc38Sx",
            'auth' : "JyGojyTGujvkxG1BRyL3zw==",
            'p256dh' : "BIS7wrOH0btq-Q7v6bGYXGAe6kjnd8s_4JkbgItcl5fs5ays0R02lqpCY-gyoKYxsx1FdRcSsZBBkBqL7QYqY6c="
        },
        'session_id': '{}'.format(uuid.uuid4()),
        'name': 'd360_push_token_update',
    }

    result = post(data, default_auth())

def send_push_opt_in():
    data = {
        'properties': {
            'channel': {
                'type' : "push",
                'opted_in' : "true"
            }
        },
        'name': 'd360_channel_settings'
    }
    result = post(data, default_auth())

def send_email_subscribe_no_email():
    data = {
        'properties': {
        },
        'name': 'd360_app_email_update'
    }
    result = post(data, default_auth())

def send_email_subscribe_empty_email():
    data = {
        'properties': {
            'email_primary' : ""
        },
        'name': 'd360_app_email_update'
    }
    result = post(data, default_auth())

def send_email_subscribe_invalid_email():
    data = {
        'properties': {
            'email_primary' : "some random string"
        },
        'name': 'd360_app_email_update'
    }
    result = post(data, default_auth())

def send_email_subscribe():
    data = {
        'properties': {
            'email_primary' : "x@y.z"
        },
        'name': 'd360_app_email_update'
    }
    result = post(data, default_auth())

def send_email_confirm():
    data = {
        'properties': {
            'channel': {
                'type' : "email",
                'opted_in' : "true"
            }
        },
        'name': 'd360_channel_settings'
    }
    result = post(data, default_auth())

def send_email_change():
    data = {
        'properties': {
            'email_primary' : "f@o.o"
        },
        'name': 'd360_app_email_update'
    }
    result = post(data, default_auth())

def send_email_confirm():
    data = {
        'properties': {
            'channel': {
                'type' : "email",
                'opted_in' : "true"
            }
        },
        'name': 'd360_channel_settings'
    }
    result = post(data, default_auth())

def send_email_unsubscribe():
    data = {
        'properties': {
            'channel': {
                'type' : "email",
                'opted_in' : "false"
            }
        },
        'name': 'd360_channel_settings'
    }
    result = post(data, default_auth())

def send_email_resubscribe():
    data = {
        'properties': {
            'channel': {
                'type' : "email",
                'opted_in' : "true"
            }
        },
        'name': 'd360_channel_settings'
    }
    result = post(data, default_auth())

def send_email_remove():
    data = {
        'name': 'd360_email_remove'
    }
    result = post(data, default_auth())



if not device_id:
    print("Registering a new device")
    device_id = get_device_id()

print (device_id)
#send_batch()
#send_empty()
